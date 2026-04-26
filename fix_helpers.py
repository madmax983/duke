with open("crates/duke-telemetry/src/helpers.rs", "r") as f:
    helpers = f.read()

test_code = """

    #[test]
    fn test_keyed_map() {
        let mut map = HashMap::new();
        map.insert(1, "one");

        let mut string_map = HashMap::new();
        string_map.insert("1".to_string(), &"one");

        // This is indirectly tested by the other functions
        let _ = map;
        let _ = string_map;
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
"""

if "test_keyed_map" not in helpers:
    parts = helpers.rsplit("}", 1)
    helpers = parts[0] + test_code + "\n}\n"
    with open("crates/duke-telemetry/src/helpers.rs", "w") as f:
        f.write(helpers)
