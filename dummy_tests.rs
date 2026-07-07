#[cfg(test)]
mod tests {
    use duke_telemetry::TelemetryStore;

    #[test]
    fn test_print_bytecode_cost_with_more_than_10() {
        let mut store = TelemetryStore::default();
        for i in 0..15 {
            store.bytecode_cost.record(&format!("op{}", i), "Foo", "bar", 10, 100);
        }
        let mut buf = Vec::new();
        store.print_bytecode_cost(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("op0"));
    }

    #[test]
    fn test_print_object_lineage_with_more_than_10() {
        let mut store = TelemetryStore::default();
        for i in 0..15 {
            store.object_lineage.record(&format!("java/lang/String{}", i), "Foo", 10, "bar");
        }
        let mut buf = Vec::new();
        store.print_object_lineage(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("java/lang/String0"));
    }

    #[test]
    fn test_print_dispatch_resolution_with_more_than_10() {
        let mut store = TelemetryStore::default();
        for i in 0..15 {
            store.dispatch_resolution.record(&format!("Foo{}", i), 42, "java/lang/String", true);
        }
        let mut buf = Vec::new();
        store.print_dispatch_resolution(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("Foo0"));
    }

    #[test]
    fn test_print_native_boundary_with_more_than_10() {
        let mut store = TelemetryStore::default();
        for i in 0..15 {
            store.native_boundary.record_call(&format!("java/lang/String{}", i), "intern", 100, true);
        }
        let mut buf = Vec::new();
        store.print_native_boundary(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("java/lang/String0"));
    }
}
