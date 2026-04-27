#[test]
fn test_helpers() {
    let map = std::collections::HashMap::<i32, i32>::new();
    let mut serializer = serde_json::Serializer::new(Vec::new());
    duke_telemetry::helpers::ser_helpers::keyed_map(&map, &mut serializer, |k| k.to_string()).unwrap();
}
