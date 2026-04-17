import re
with open("crates/duke-telemetry/src/lib.rs", "r") as f:
    content = f.read()

replacement = """
    #[test]
    #[cfg(feature = "telemetry")]
    fn telemetry_store_print_report() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store
            .class_init_dag
            .record("java/lang/String", "java/lang/System", 500);

        store.object_lineage.record("org/MyClass", "myMethod", 10, "org/MyObject");
        store.exception_flow.record("java/lang/Exception", ("Foo".to_string(), "bar".to_string(), 10));
        store.exception_flow.events[0].catch_site = Some(("Foo".to_string(), "baz".to_string(), 20));
        store.dispatch_resolution.record("Foo", 10, "Bar", 2);
        store.native_boundary.record("Foo", "bar", 500, false);

        let mut buf = Vec::new();
        store.print_report(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("=== Duke VM Telemetry Report ==="));
        assert!(s.contains("iadd"));
        assert!(s.contains("java/lang/String"));
        assert!(s.contains("java/lang/System"));
        assert!(s.contains("org/MyClass"));
        assert!(s.contains("java/lang/Exception"));
        assert!(s.contains("Foo[cp10]"));
        assert!(s.contains("Foo.bar"));
    }
"""

content = re.sub(r'    #\[test\]\n    #\[cfg\(feature = "telemetry"\)\]\n    fn telemetry_store_print_report\(\) \{[\s\S]*?    \}\n', replacement, content, flags=re.MULTILINE)

with open("crates/duke-telemetry/src/lib.rs", "w") as f:
    f.write(content)
