#[cfg(test)]
#[cfg(feature = "telemetry")]
mod tests {
    use duke_telemetry::ser_helpers::*;
    use serde::Serialize;
    use std::collections::HashMap;

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
        #[serde(serialize_with = "site3")]
        map: HashMap<(String, String, usize), FailingDummy>,
    }

    #[test]
    fn test_keyed_map_error_site3() {
        let mut map = HashMap::new();
        map.insert(
            ("java/lang/String".to_string(), "intern".to_string(), 42),
            FailingDummy { val: 1 },
        );
        let w = Site3FailingWrapper { map };
        let res = serde_json::to_string(&w);
        assert!(res.is_err());
    }
}
