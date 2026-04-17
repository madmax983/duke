with open("crates/duke-telemetry/src/lib.rs", "r") as f:
    content = f.read()

content = content.replace("fn telemetry_store_print_report() {", """fn telemetry_store_to_json() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        let json = store.to_json();
        assert!(json.contains("iadd"));
    }

    #[test]
    fn telemetry_store_print_report() {""")

with open("crates/duke-telemetry/src/lib.rs", "w") as f:
    f.write(content)
