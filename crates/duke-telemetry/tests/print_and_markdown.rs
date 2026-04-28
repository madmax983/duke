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
