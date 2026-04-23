use duke_telemetry::TelemetryStore;

#[test]
fn test_telemetry_new() {
    let _store = TelemetryStore::new();
}

#[test]
fn test_telemetry_print() {
    let mut store = TelemetryStore::new();
    store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
    store
        .object_lineage
        .record("java/lang/String", "Foo", 10, "bar");
    store
        .class_init_dag
        .record("java/lang/String", "java/lang/System", 500);
    let idx = store
        .exception_flow
        .record_throw("java/lang/Exception", "Foo", "bar", 10);
    store.exception_flow.record_catch(idx, "Foo", "bar", 20);
    store
        .dispatch_resolution
        .record("Foo", 42, "java/lang/String", true);
    store
        .native_boundary
        .record_call("java/lang/String", "intern", 100, true);

    let mut buf = Vec::new();
    store.print_report(&mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("=== Duke VM Telemetry Report ==="));
    assert!(output.contains("iadd"));
    assert!(output.contains("java/lang/String"));
}

#[test]
fn test_telemetry_markdown() {
    let mut store = TelemetryStore::new();
    store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
    store
        .object_lineage
        .record("java/lang/String", "Foo", 10, "bar");
    store
        .class_init_dag
        .record("java/lang/String", "java/lang/System", 500);
    let idx = store
        .exception_flow
        .record_throw("java/lang/Exception", "Foo", "bar", 10);
    store.exception_flow.record_catch(idx, "Foo", "bar", 20);
    store
        .dispatch_resolution
        .record("Foo", 42, "java/lang/String", true);
    store
        .native_boundary
        .record_call("java/lang/String", "intern", 100, true);

    let output = store.to_markdown_report();
    assert!(output.contains("# Duke VM Telemetry Report"));
    assert!(output.contains("iadd"));
    assert!(output.contains("java/lang/String"));
}

#[test]
fn test_telemetry_markdown_empty() {
    let store = TelemetryStore::new();
    let output = store.to_markdown_report();
    assert!(output.contains("# Duke VM Telemetry Report"));
    assert!(output.contains("No class initialization events recorded."));
}

#[test]
#[cfg(feature = "telemetry")]
fn test_telemetry_json() {
    let mut store = TelemetryStore::new();
    store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
    let output = store.to_json();
    assert!(output.contains("bytecode_cost"));
}

#[test]
fn test_telemetry_print_uncaught() {
    let mut store = TelemetryStore::new();
    let _idx = store
        .exception_flow
        .record_throw("java/lang/Exception", "Foo", "bar", 10);
    // leave uncaught
    let mut buf = Vec::new();
    store.print_report(&mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("=== Duke VM Telemetry Report ==="));
    assert!(output.contains("uncaught"));
}

#[test]
fn test_telemetry_markdown_uncaught() {
    let mut store = TelemetryStore::new();
    let _idx = store
        .exception_flow
        .record_throw("java/lang/Exception", "Foo", "bar", 10);
    let output = store.to_markdown_report();
    assert!(output.contains("# Duke VM Telemetry Report"));
    assert!(output.contains("uncaught"));
}
