import re

with open("crates/duke-telemetry/src/lib.rs", "r") as f:
    content = f.read()

# Replace all modules at the bottom
new_test_block = """
#[cfg(test)]
mod tests {
    #[cfg(feature = "telemetry")]
    use crate::TelemetryStore;

    #[test]
    #[cfg(feature = "telemetry")]
    fn telemetry_store_to_markdown_report() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store
            .class_init_dag
            .record("java/lang/String", "java/lang/System", 500);

        let md = store.to_markdown_report();
        assert!(md.contains("# Duke VM Telemetry Report"));
        assert!(md.contains("| `iadd` | 1 | 100 |"));
        assert!(md.contains("```mermaid\\n"));
        assert!(md.contains("graph TD;\\n"));
        assert!(md.contains("\\"java/lang/System\\" -->|500ns| \\"java/lang/String\\";"));
        assert!(md.contains("```"));
    }

    #[test]
    #[cfg(feature = "telemetry")]
    fn should_indicate_empty_class_initialization_in_markdown_report() {
        let empty_store = TelemetryStore::default();
        let empty_md = empty_store.to_markdown_report();
        assert!(empty_md.contains("No class initialization events recorded."));
    }

    #[test]
    #[cfg(feature = "telemetry")]
    fn should_correctly_format_print_report_with_populated_data() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store.object_lineage.record("java/lang/String", "Foo", 10, "bar");
        store.class_init_dag.record("java/lang/String", "java/lang/System", 500);
        store.exception_flow.record_throw("java/lang/Exception", "Foo", "bar", 10);
        store.dispatch_resolution.record("Foo", 42, "java/lang/String", true);
        store.dispatch_resolution.record("Foo", 42, "java/lang/String", true);
        store.dispatch_resolution.record("Foo", 42, "java/lang/String", true);
        store.native_boundary.record_call("java/lang/String", "intern", 100, true);

        let mut buf = Vec::new();
        store.print_report(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();

        assert!(s.contains("=== Duke VM Telemetry Report ==="));
        assert!(s.contains("iadd                 count=         1"));
        assert!(s.contains("allocs 1 of bar"));
        assert!(s.contains("java/lang/String (triggered by: java/lang/System, 500ns)"));
        assert!(s.contains("java/lang/Exception thrown at (\\"Foo\\", \\"bar\\", 10) caught at uncaught"));
        assert!(s.contains("Foo[cp42] calls=3 targets=1 walks=3"));
        assert!(s.contains("java/lang/String.intern calls=1 errors=1"));
    }

    #[test]
    #[cfg(feature = "telemetry")]
    fn should_correctly_serialize_telemetry_store_to_json() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store.object_lineage.record("java/lang/String", "Foo", 10, "bar");
        store.class_init_dag.record("java/lang/String", "java/lang/System", 500);
        store.exception_flow.record_throw("java/lang/Exception", "Foo", "bar", 10);
        store.dispatch_resolution.record("Foo", 42, "java/lang/String", true);
        store.native_boundary.record_call("java/lang/String", "intern", 100, true);

        let json = store.to_json();
        assert!(json.contains("\\"bytecode_cost\\""));
        assert!(json.contains("\\"iadd\\""));
        assert!(json.contains("\\"object_lineage\\""));
        assert!(json.contains("\\"java/lang/String::Foo@10\\""));
        assert!(json.contains("\\"class_init_dag\\""));
        assert!(json.contains("\\"java/lang/String\\""));
        assert!(json.contains("\\"exception_flow\\""));
        assert!(json.contains("\\"java/lang/Exception\\""));
        assert!(json.contains("\\"dispatch_resolution\\""));
        assert!(json.contains("\\"Foo@42\\""));
        assert!(json.contains("\\"native_boundary\\""));
        assert!(json.contains("\\"java/lang/String::intern\\""));
    }
}
"""

# Replace everything from `#[cfg(test)]\nmod tests {` to the end of the file.
content = re.sub(r'#\[cfg\(test\)\]\nmod tests \{.*', new_test_block, content, flags=re.DOTALL)

with open("crates/duke-telemetry/src/lib.rs", "w") as f:
    f.write(content)
