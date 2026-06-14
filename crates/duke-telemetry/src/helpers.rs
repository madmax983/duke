//! Serde serialization helpers for telemetry data structures.
#[cfg(feature = "telemetry")]
pub mod ser_helpers {
    use std::collections::HashMap;

    use serde::{Serialize, ser::SerializeMap};

    /// Core helper: serialize any `HashMap<K, V>` by formatting each key using a zero-cost wrapper.
    ///
    /// ⚡ Bolt: Using `SerializeMap` and a custom display struct avoids `format!` string allocations.
    pub fn keyed_map<'a, K, V, S, F, W>(map: &'a HashMap<K, V>, ser: S, key_fn: F) -> Result<S::Ok, S::Error>
    where
        K: Eq + std::hash::Hash + 'a,
        V: Serialize,
        S: serde::Serializer,
        F: Fn(&'a K) -> W,
        W: Serialize,
    {
        let mut map_ser = ser.serialize_map(Some(map.len()))?;
        for (k, v) in map {
            map_ser.serialize_entry(&key_fn(k), v)?;
        }
        map_ser.end()
    }

    struct Site3Key<'a>(&'a String, &'a String, usize);
    impl std::fmt::Display for Site3Key<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}::{}@{}", self.0, self.1, self.2)
        }
    }
    impl Serialize for Site3Key<'_> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.collect_str(self)
        }
    }

    /// `HashMap<(class, method, pc), V>` → `"class::method@pc"`.
    pub fn site3<V: Serialize, S: serde::Serializer>(
        map: &HashMap<(String, String, usize), V>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        keyed_map(map, ser, |(c, m, pc)| Site3Key(c, m, *pc))
    }

    struct Site2U16Key<'a>(&'a String, u16);
    impl std::fmt::Display for Site2U16Key<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}@{}", self.0, self.1)
        }
    }
    impl Serialize for Site2U16Key<'_> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.collect_str(self)
        }
    }

    /// `HashMap<(class, cp_idx), V>` → `"class@cp"`.
    pub fn site2_u16<V: Serialize, S: serde::Serializer>(
        map: &HashMap<(String, u16), V>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        keyed_map(map, ser, |(c, cp)| Site2U16Key(c, *cp))
    }

    struct PairStrKey<'a>(&'a String, &'a String);
    impl std::fmt::Display for PairStrKey<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}::{}", self.0, self.1)
        }
    }
    impl Serialize for PairStrKey<'_> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.collect_str(self)
        }
    }

    /// `HashMap<(class, method), V>` → `"class::method"`.
    pub fn pair_str<V: Serialize, S: serde::Serializer>(
        map: &HashMap<(String, String), V>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        keyed_map(map, ser, |(c, m)| PairStrKey(c, m))
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

    struct CustomKey<'a>(&'a i32);
    impl std::fmt::Display for CustomKey<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "key_{}", self.0)
        }
    }
    impl Serialize for CustomKey<'_> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.collect_str(self)
        }
    }

    fn custom_keyed_map<S: serde::Serializer>(
        map: &HashMap<i32, &'static str>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        super::ser_helpers::keyed_map(map, ser, CustomKey)
    }

    #[test]
    fn test_empty_serialize() {
        let empty_map = HashMap::<i32, &'static str>::new();
        let w = MapWrapper { map: empty_map };
        let json = serde_json::to_string(&w).unwrap();
        assert_eq!(json, r#"{"map":{}}"#);
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
        let json = serde_json::to_string(&empty_site3).unwrap();
        assert_eq!(json, r#"{"map":{}}"#);

        let empty_site2 = Site2Wrapper {
            map: HashMap::new(),
        };
        let json = serde_json::to_string(&empty_site2).unwrap();
        assert_eq!(json, r#"{"map":{}}"#);

        let empty_pair = PairWrapper {
            map: HashMap::new(),
        };
        let json = serde_json::to_string(&empty_pair).unwrap();
        assert_eq!(json, r#"{"map":{}}"#);

        let empty_set = SetWrapper {
            set: HashSet::new(),
        };
        let json = serde_json::to_string(&empty_set).unwrap();
        assert_eq!(json, r#"{"set":[]}"#);
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

    #[derive(Serialize)]
    struct FailingDummy {
        #[serde(serialize_with = "failing_serialize")]
        val: i32,
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn failing_serialize<S: serde::Serializer>(_val: &i32, _ser: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("test error"))
    }

    #[derive(Serialize)]
    struct Site3FailingWrapper {
        #[serde(serialize_with = "super::ser_helpers::site3")]
        map: HashMap<(String, String, usize), FailingDummy>,
    }

    #[derive(Serialize)]
    struct Site2FailingWrapper {
        #[serde(serialize_with = "super::ser_helpers::site2_u16")]
        map: HashMap<(String, u16), FailingDummy>,
    }

    #[derive(Serialize)]
    struct PairFailingWrapper {
        #[serde(serialize_with = "super::ser_helpers::pair_str")]
        map: HashMap<(String, String), FailingDummy>,
    }

    #[derive(Serialize)]
    struct CustomKeyedMapFailingWrapper {
        #[serde(serialize_with = "failing_custom_keyed_map")]
        map: HashMap<i32, FailingDummy>,
    }

    fn failing_custom_keyed_map<S: serde::Serializer>(
        map: &HashMap<i32, FailingDummy>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        super::ser_helpers::keyed_map(map, ser, CustomKey)
    }

    #[test]
    fn test_keyed_map_error() {
        let mut map = HashMap::new();
        map.insert(
            ("java/lang/String".to_string(), "intern".to_string(), 42),
            FailingDummy { val: 1 },
        );
        let w = Site3FailingWrapper { map };
        let res = serde_json::to_string(&w);
        assert!(res.is_err());
    }

    #[test]
    fn test_site2_u16_error() {
        let mut map = HashMap::new();
        map.insert(
            ("java/lang/String".to_string(), 42),
            FailingDummy { val: 1 },
        );
        let w = Site2FailingWrapper { map };
        let res = serde_json::to_string(&w);
        assert!(res.is_err());
    }

    #[test]
    fn test_pair_str_error() {
        let mut map = HashMap::new();
        map.insert(
            ("java/lang/String".to_string(), "intern".to_string()),
            FailingDummy { val: 1 },
        );
        let w = PairFailingWrapper { map };
        let res = serde_json::to_string(&w);
        assert!(res.is_err());
    }

    #[test]
    fn test_custom_keyed_map_error() {
        let mut map = HashMap::new();
        map.insert(1, FailingDummy { val: 1 });
        let w = CustomKeyedMapFailingWrapper { map };
        let res = serde_json::to_string(&w);
        assert!(res.is_err());
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_sorted_set_error() {
        struct FailingSerializer;
        impl serde::Serializer for FailingSerializer {
            type Ok = ();
            type Error = serde::de::value::Error;
            type SerializeSeq = serde::ser::Impossible<(), Self::Error>;
            type SerializeTuple = serde::ser::Impossible<(), Self::Error>;
            type SerializeTupleStruct = serde::ser::Impossible<(), Self::Error>;
            type SerializeTupleVariant = serde::ser::Impossible<(), Self::Error>;
            type SerializeMap = serde::ser::Impossible<(), Self::Error>;
            type SerializeStruct = serde::ser::Impossible<(), Self::Error>;
            type SerializeStructVariant = serde::ser::Impossible<(), Self::Error>;
            fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_some<T: ?Sized + Serialize>(
                self,
                _v: &T,
            ) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_unit_variant(
                self,
                _n: &'static str,
                _vi: u32,
                _va: &'static str,
            ) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_newtype_struct<T: ?Sized + Serialize>(
                self,
                _name: &'static str,
                _v: &T,
            ) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_newtype_variant<T: ?Sized + Serialize>(
                self,
                _n: &'static str,
                _vi: u32,
                _va: &'static str,
                _v: &T,
            ) -> Result<Self::Ok, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_tuple_struct(
                self,
                _name: &'static str,
                _len: usize,
            ) -> Result<Self::SerializeTupleStruct, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_tuple_variant(
                self,
                _n: &'static str,
                _vi: u32,
                _va: &'static str,
                _len: usize,
            ) -> Result<Self::SerializeTupleVariant, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
                Err(serde::de::Error::custom("map err"))
            }
            fn serialize_struct(
                self,
                _name: &'static str,
                _len: usize,
            ) -> Result<Self::SerializeStruct, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
            fn serialize_struct_variant(
                self,
                _n: &'static str,
                _vi: u32,
                _va: &'static str,
                _len: usize,
            ) -> Result<Self::SerializeStructVariant, Self::Error> {
                Err(serde::de::Error::custom("err"))
            }
        }

        let mut set = std::collections::HashSet::new();
        set.insert("a".to_string());
        let res = super::ser_helpers::sorted_set(&set, FailingSerializer);
        assert!(res.is_err());
    }
}
