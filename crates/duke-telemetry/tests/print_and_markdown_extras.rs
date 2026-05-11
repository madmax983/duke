use duke_telemetry::TelemetryStore;

#[test]
fn should_populate_dispatch_resolution_print_report() {
    let mut store = TelemetryStore::default();
    store
        .dispatch_resolution
        .record("Foo", 42, "java/lang/String", true);
    let mut buf = Vec::new();
    store.print_report(&mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("Foo"));
}

#[test]
fn should_populate_native_boundary_print_report() {
    let mut store = TelemetryStore::default();
    store
        .native_boundary
        .record_call("java/lang/String", "intern", 100, true);
    let mut buf = Vec::new();
    store.print_report(&mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("java/lang/String"));
}

#[test]
fn should_populate_dispatch_resolution_markdown_report() {
    let mut store = TelemetryStore::default();
    store
        .dispatch_resolution
        .record("Foo", 42, "java/lang/String", true);
    let s = store.to_markdown_report();
    assert!(s.contains("Foo"));
}

#[test]
fn should_populate_native_boundary_markdown_report() {
    let mut store = TelemetryStore::default();
    store
        .native_boundary
        .record_call("java/lang/String", "intern", 100, true);
    let s = store.to_markdown_report();
    assert!(s.contains("java/lang/String"));
}

#[test]
fn should_populate_dispatch_resolution_markdown_empty() {
    let store = TelemetryStore::default();
    let md = store.to_markdown_report();
    assert!(md.contains("## Dispatch Resolution (Top 10 Virtual Call Sites)"));
}

#[test]
fn should_populate_native_boundary_markdown_empty() {
    let store = TelemetryStore::default();
    let md = store.to_markdown_report();
    assert!(md.contains("## Native Boundary (Top 10 by Call Count)"));
}
