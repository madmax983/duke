#[cfg(feature = "telemetry")]
pub mod ser_helpers {
    use std::collections::HashMap;

    use serde::Serialize;

    /// Core helper: serialize any `HashMap<K, V>` by formatting each key with `key_fn`.
    fn keyed_map<K, V, S, F>(map: &HashMap<K, V>, ser: S, key_fn: F) -> Result<S::Ok, S::Error>
    where
        K: Eq + std::hash::Hash,
        V: Serialize,
        S: serde::Serializer,
        F: Fn(&K) -> String,
    {
        let string_map: HashMap<String, &V> = map.iter().map(|(k, v)| (key_fn(k), v)).collect();
        string_map.serialize(ser)
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
    struct DummyVal(i32);

    #[derive(Serialize)]
    struct Site3Test {
        #[serde(serialize_with = "site3")]
        map: HashMap<(String, String, usize), DummyVal>,
    }

    #[test]
    fn test_site3_serialization() {
        let mut map = HashMap::new();
        map.insert(
            ("ClassA".to_string(), "methodB".to_string(), 42),
            DummyVal(1),
        );
        let test_obj = Site3Test { map };
        let json = serde_json::to_string(&test_obj).unwrap();
        assert_eq!(json, r#"{"map":{"ClassA::methodB@42":1}}"#);
    }

    #[derive(Serialize)]
    struct Site2U16Test {
        #[serde(serialize_with = "site2_u16")]
        map: HashMap<(String, u16), DummyVal>,
    }

    #[test]
    fn test_site2_u16_serialization() {
        let mut map = HashMap::new();
        map.insert(("ClassA".to_string(), 42), DummyVal(2));
        let test_obj = Site2U16Test { map };
        let json = serde_json::to_string(&test_obj).unwrap();
        assert_eq!(json, r#"{"map":{"ClassA@42":2}}"#);
    }

    #[derive(Serialize)]
    struct PairStrTest {
        #[serde(serialize_with = "pair_str")]
        map: HashMap<(String, String), DummyVal>,
    }

    #[test]
    fn test_pair_str_serialization() {
        let mut map = HashMap::new();
        map.insert(("ClassA".to_string(), "methodB".to_string()), DummyVal(3));
        let test_obj = PairStrTest { map };
        let json = serde_json::to_string(&test_obj).unwrap();
        assert_eq!(json, r#"{"map":{"ClassA::methodB":3}}"#);
    }

    #[derive(Serialize)]
    struct SortedSetTest {
        #[serde(serialize_with = "sorted_set")]
        set: HashSet<String>,
    }

    #[test]
    fn test_sorted_set_serialization() {
        let mut set = HashSet::new();
        set.insert("B".to_string());
        set.insert("C".to_string());
        set.insert("A".to_string());
        let test_obj = SortedSetTest { set };
        let json = serde_json::to_string(&test_obj).unwrap();
        // Since we sort the set when serializing, output should be guaranteed
        assert_eq!(json, r#"{"set":["A","B","C"]}"#);
    }
}
