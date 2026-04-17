with open("crates/duke-telemetry/src/lib.rs", "r") as f:
    content = f.read()

replacement = """    #[test]
    fn telemetry_store_print_report() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store
            .class_init_dag
            .record("java/lang/String", "java/lang/System", 500);

        store.object_lineage.record("org/MyClass", "myMethod", 10, "org/MyObject");
        let exc_idx = store.exception_flow.record_throw("java/lang/Exception", "Foo", "bar", 10);
        store.exception_flow.record_catch(exc_idx, "Foo", "baz", 20);
        store.dispatch_resolution.record("Foo", 10, "Bar", true);
        store.native_boundary.record_call("Foo", "bar", 500, false);

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

import re
content = re.sub(r'    #\[test\]\n    fn telemetry_store_print_report\(\) \{[\s\S]*?    \}\n', replacement, content, flags=re.MULTILINE)

with open("crates/duke-telemetry/src/lib.rs", "w") as f:
    f.write(content)
