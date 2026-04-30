//! Serde serialization helpers for telemetry data structures.
#[cfg(feature = "telemetry")]
pub(crate) mod ser_helpers {
    use std::collections::HashMap;

    use serde::{Serialize, ser::SerializeMap};

    /// Core helper: serialize any `HashMap<K, V>` by formatting each key with `key_fn`.
    ///
    /// ⚡ Bolt: Using `SerializeMap` to serialize directly removes the intermediate `HashMap`
    /// collection, avoiding heap allocations and hashing overhead during telemetry generation.
    pub fn keyed_map<K, V, S, F>(map: &HashMap<K, V>, ser: S, key_fn: F) -> Result<S::Ok, S::Error>
    where
        K: Eq + std::hash::Hash,
        V: Serialize,
        S: serde::Serializer,
        F: Fn(&K) -> String,
    {
        let mut map_ser = ser.serialize_map(Some(map.len()))?;
        for (k, v) in map {
            map_ser.serialize_entry(&key_fn(k), v)?;
        }
        map_ser.end()
    }

    /// `HashMap<(class, method, pc), V>` → `"class::method@pc"`.
    pub fn site3<V: Serialize, S: serde::Serializer>(
        map: &HashMap<(String, String, usize), V>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        keyed_map(map, ser, |(c, m, pc)| format!("{c}::{m}@{pc}"))
    }

    /// `HashMap<(class, cp_idx), V>` → `"class@cp"`.
    pub fn site2_u16<V: Serialize, S: serde::Serializer>(
        map: &HashMap<(String, u16), V>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        keyed_map(map, ser, |(c, cp)| format!("{c}@{cp}"))
    }

    /// `HashMap<(class, method), V>` → `"class::method"`.
    pub fn pair_str<V: Serialize, S: serde::Serializer>(
        map: &HashMap<(String, String), V>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        keyed_map(map, ser, |(c, m)| format!("{c}::{m}"))
    }

    /// Serialize `HashSet<String>` as a sorted `Vec<String>` for deterministic output.
    pub fn sorted_set<S: serde::Serializer>(
        set: &std::collections::HashSet<String>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        let mut v: Vec<&String> = set.iter().collect();
        v.sort();
        v.serialize(ser)
    }
}
#[cfg(test)]
#[cfg(feature = "telemetry")]
mod tests {
    use super::ser_helpers::*;
    use serde::Serialize;
    use std::collections::{HashMap, HashSet};

    #[derive(Serialize)]
    struct Dummy {
        val: i32,
    }

    #[derive(Serialize)]
    struct Site3Wrapper {
        #[serde(serialize_with = "site3")]
        map: HashMap<(String, String, usize), Dummy>,
    }

    #[derive(Serialize)]
    struct Site2Wrapper {
        #[serde(serialize_with = "site2_u16")]
        map: HashMap<(String, u16), Dummy>,
    }

    #[derive(Serialize)]
    struct PairWrapper {
        #[serde(serialize_with = "pair_str")]
        map: HashMap<(String, String), Dummy>,
    }

    #[derive(Serialize)]
    struct SetWrapper {
        #[serde(serialize_with = "sorted_set")]
        set: HashSet<String>,
    }

    #[derive(Serialize)]
    struct MapWrapper {
        #[serde(serialize_with = "custom_keyed_map")]
        map: HashMap<i32, &'static str>,
    }

    fn custom_keyed_map<S: serde::Serializer>(
        map: &HashMap<i32, &'static str>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        super::ser_helpers::keyed_map(map, ser, |k| format!("key_{k}"))
    }

    #[test]
    fn test_keyed_map() {
        let mut map = HashMap::new();
        map.insert(1, "one");
        map.insert(2, "two");

        let w = MapWrapper { map };
        let json = serde_json::to_string(&w).unwrap();
        assert!(json.contains("\"key_1\":\"one\""));
        assert!(json.contains("\"key_2\":\"two\""));
    }

    #[test]
    fn test_empty_maps() {
        let empty_map = HashMap::<i32, &'static str>::new();
        let w = MapWrapper { map: empty_map };
        let json = serde_json::to_string(&w).unwrap();
        assert_eq!(json, r#"{"map":{}}"#);

        let empty_site3 = Site3Wrapper {
            map: HashMap::new(),
        };
        let json_site3 = serde_json::to_string(&empty_site3).unwrap();
        assert_eq!(json_site3, r#"{"map":{}}"#);

        let empty_site2 = Site2Wrapper {
            map: HashMap::new(),
        };
        let json_site2 = serde_json::to_string(&empty_site2).unwrap();
        assert_eq!(json_site2, r#"{"map":{}}"#);

        let empty_pair = PairWrapper {
            map: HashMap::new(),
        };
        let json_pair = serde_json::to_string(&empty_pair).unwrap();
        assert_eq!(json_pair, r#"{"map":{}}"#);

        let empty_set = SetWrapper {
            set: HashSet::new(),
        };
        let json_set = serde_json::to_string(&empty_set).unwrap();
        assert_eq!(json_set, r#"{"set":[]}"#);
    }

    #[test]
    fn test_site3() {
        let mut map = HashMap::new();
        map.insert(
            ("java/lang/String".to_string(), "intern".to_string(), 42),
            Dummy { val: 1 },
        );

        let w = Site3Wrapper { map };
        let json = serde_json::to_string(&w).unwrap();
        assert_eq!(json, r#"{"map":{"java/lang/String::intern@42":{"val":1}}}"#);
    }

    #[test]
    fn test_site2_u16() {
        let mut map = HashMap::new();
        map.insert(("java/lang/String".to_string(), 42), Dummy { val: 1 });

        let w = Site2Wrapper { map };
        let json = serde_json::to_string(&w).unwrap();
        assert_eq!(json, r#"{"map":{"java/lang/String@42":{"val":1}}}"#);
    }

    #[test]
    fn test_pair_str() {
        let mut map = HashMap::new();
        map.insert(
            ("java/lang/String".to_string(), "intern".to_string()),
            Dummy { val: 1 },
        );

        let w = PairWrapper { map };
        let json = serde_json::to_string(&w).unwrap();
        assert_eq!(json, r#"{"map":{"java/lang/String::intern":{"val":1}}}"#);
    }

    #[test]
    fn test_sorted_set() {
        let mut set = HashSet::new();
        set.insert("b".to_string());
        set.insert("a".to_string());
        set.insert("c".to_string());

        let w = SetWrapper { set };
        let json = serde_json::to_string(&w).unwrap();
        assert_eq!(json, r#"{"set":["a","b","c"]}"#);
    }
}
