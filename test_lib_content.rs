    #[test]
    #[cfg(feature = "telemetry")]
    fn telemetry_store_print_report() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store
            .class_init_dag
            .record("java/lang/String", "java/lang/System", 500);

        let mut buf = Vec::new();
        store.print_report(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("=== Duke VM Telemetry Report ==="));
        assert!(s.contains("iadd"));
        assert!(s.contains("java/lang/String"));
        assert!(s.contains("java/lang/System"));
    }
