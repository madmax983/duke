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

#[cfg(all(test, feature = "telemetry"))]
mod tests {
    use super::ser_helpers::*;
    use serde::Serialize;
    use std::collections::{HashMap, HashSet};

    #[derive(Serialize)]
    struct DummyStruct {
        value: i32,
    }

    #[test]
    fn test_site3() {
        let mut map = HashMap::new();
        map.insert(
            ("class1".to_string(), "method1".to_string(), 42),
            DummyStruct { value: 10 },
        );
        let json = serde_json::to_string(&serde_helpers_wrapper_site3(&map)).unwrap();
        assert_eq!(json, "{\"class1::method1@42\":{\"value\":10}}");
    }

    struct Site3Wrapper<'a>(&'a HashMap<(String, String, usize), DummyStruct>);
    impl Serialize for Site3Wrapper<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            site3(self.0, serializer)
        }
    }
    const fn serde_helpers_wrapper_site3(
        map: &HashMap<(String, String, usize), DummyStruct>,
    ) -> Site3Wrapper<'_> {
        Site3Wrapper(map)
    }

    #[test]
    fn test_site2_u16() {
        let mut map = HashMap::new();
        map.insert(("class1".to_string(), 42), DummyStruct { value: 10 });
        let json = serde_json::to_string(&serde_helpers_wrapper_site2_u16(&map)).unwrap();
        assert_eq!(json, "{\"class1@42\":{\"value\":10}}");
    }

    struct Site2U16Wrapper<'a>(&'a HashMap<(String, u16), DummyStruct>);
    impl Serialize for Site2U16Wrapper<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            site2_u16(self.0, serializer)
        }
    }
    const fn serde_helpers_wrapper_site2_u16(
        map: &HashMap<(String, u16), DummyStruct>,
    ) -> Site2U16Wrapper<'_> {
        Site2U16Wrapper(map)
    }

    #[test]
    fn test_pair_str() {
        let mut map = HashMap::new();
        map.insert(
            ("class1".to_string(), "method1".to_string()),
            DummyStruct { value: 10 },
        );
        let json = serde_json::to_string(&serde_helpers_wrapper_pair_str(&map)).unwrap();
        assert_eq!(json, "{\"class1::method1\":{\"value\":10}}");
    }

    struct PairStrWrapper<'a>(&'a HashMap<(String, String), DummyStruct>);
    impl Serialize for PairStrWrapper<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            pair_str(self.0, serializer)
        }
    }
    const fn serde_helpers_wrapper_pair_str(
        map: &HashMap<(String, String), DummyStruct>,
    ) -> PairStrWrapper<'_> {
        PairStrWrapper(map)
    }

    #[test]
    fn test_sorted_set() {
        let mut set = HashSet::new();
        set.insert("b".to_string());
        set.insert("a".to_string());
        set.insert("c".to_string());
        let json = serde_json::to_string(&serde_helpers_wrapper_sorted_set(&set)).unwrap();
        assert_eq!(json, "[\"a\",\"b\",\"c\"]");
    }

    struct SortedSetWrapper<'a>(&'a HashSet<String>);
    impl Serialize for SortedSetWrapper<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            sorted_set(self.0, serializer)
        }
    }
    const fn serde_helpers_wrapper_sorted_set(set: &HashSet<String>) -> SortedSetWrapper<'_> {
        SortedSetWrapper(set)
    }
}
