use duke_telemetry::TelemetryStore;

#[test]
fn test_print_bytecode_cost_empty() {
    let store = TelemetryStore::new();
    let mut buf = Vec::new();
    store.print_report(&mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("-- bytecode_cost (top 10 by count) --"));
}

#[test]
fn test_markdown_bytecode_cost_empty() {
    let store = TelemetryStore::new();
    let output = store.to_markdown_report();
    assert!(output.contains("## Bytecode Cost (Top 10)"));
}

#[test]
fn test_print_and_markdown_reports() {
    let mut store = TelemetryStore::default();

    // Add data to cover the loops
    for i in 0..15 {
        let op = format!("op{i}");
        store
            .bytecode_cost
            .record(Box::leak(op.into_boxed_str()), "Foo", "bar", i, 100);

        let class = format!("Class{i}");
        store.object_lineage.record(&class, "Foo", i, "bar");

        store.class_init_dag.record(&class, "System", 500);

        store.exception_flow.record_throw(&class, "Foo", "bar", i);

        store
            .dispatch_resolution
            .record("Foo", u16::try_from(i).unwrap(), "bar", i != 0);

        store
            .native_boundary
            .record_call(&class, "Foo", 100, i != 0);
    }

    let mut out = Vec::new();
    store.print_report(&mut out).unwrap();

    let md = store.to_markdown_report();
    assert!(!md.is_empty());
}
