#[cfg(test)]
#[cfg(feature = "telemetry")]
mod tests {
    use super::ser_helpers::*;
    use std::collections::{HashMap, HashSet};
    use serde::Serialize;

    #[derive(Serialize)]
    struct DummyStruct {
        value: i32,
    }

    #[test]
    fn test_site3() {
        let mut map = HashMap::new();
        map.insert(("class1".to_string(), "method1".to_string(), 42), DummyStruct { value: 10 });
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
    fn serde_helpers_wrapper_site3(map: &HashMap<(String, String, usize), DummyStruct>) -> Site3Wrapper<'_> {
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
    fn serde_helpers_wrapper_site2_u16(map: &HashMap<(String, u16), DummyStruct>) -> Site2U16Wrapper<'_> {
        Site2U16Wrapper(map)
    }

    #[test]
    fn test_pair_str() {
        let mut map = HashMap::new();
        map.insert(("class1".to_string(), "method1".to_string()), DummyStruct { value: 10 });
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
    fn serde_helpers_wrapper_pair_str(map: &HashMap<(String, String), DummyStruct>) -> PairStrWrapper<'_> {
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
    fn serde_helpers_wrapper_sorted_set(set: &HashSet<String>) -> SortedSetWrapper<'_> {
        SortedSetWrapper(set)
    }
}
