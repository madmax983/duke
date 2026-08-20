use duke_telemetry::TelemetryStore;

#[test]
fn test_telemetry_print() {
    let mut store = TelemetryStore::new();
    store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
    store
        .object_lineage
        .record("com/example/Main", "run", 42, "java/lang/String");
    store
        .class_init_dag
        .record("java/lang/String", "java/lang/System", 500);
    store
        .exception_flow
        .record_throw("java/lang/Exception", "Foo", "bar", 10);
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
    assert!(output.contains("-- bytecode_cost"));
    assert!(output.contains("iadd"));
    assert!(output.contains("-- object_lineage"));
    assert!(output.contains("java/lang/String"));
    assert!(output.contains("-- class_init_dag"));
    assert!(output.contains("java/lang/String (triggered by: java/lang/System"));
    assert!(output.contains("-- exception_flow"));
    assert!(output.contains("java/lang/Exception thrown at"));
    assert!(output.contains("-- dispatch_resolution"));
    assert!(output.contains("Foo[cp42] calls="));
    assert!(output.contains("-- native_boundary"));
    assert!(output.contains("java/lang/String.intern"));
}

#[test]
fn test_telemetry_print_uncaught() {
    let mut store = TelemetryStore::new();
    store
        .exception_flow
        .record_throw("java/lang/Exception", "Foo", "bar", 10);

    let mut buf = Vec::new();
    store.print_report(&mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("caught at uncaught"));
}

#[test]
fn test_telemetry_markdown() {
    let mut store = TelemetryStore::new();
    store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
    store
        .object_lineage
        .record("com/example/Main", "run", 42, "java/lang/String");
    store
        .class_init_dag
        .record("java/lang/String", "java/lang/System", 500);
    store
        .exception_flow
        .record_throw("java/lang/Exception", "Foo", "bar", 10);
    store
        .dispatch_resolution
        .record("Foo", 42, "java/lang/String", true);
    store
        .native_boundary
        .record_call("java/lang/String", "intern", 100, true);

    let md = store.to_markdown_report();

    assert!(md.contains("# Duke VM Telemetry Report"));
    assert!(md.contains("## Bytecode Cost (Top 10)"));
    assert!(md.contains("`iadd`"));
    assert!(md.contains("## Object Lineage"));
    assert!(md.contains("`java/lang/String`"));
    assert!(md.contains("## Class Initialization DAG"));
    assert!(md.contains("```mermaid"));
    assert!(md.contains("## Exception Flow"));
    assert!(md.contains("`java/lang/Exception`"));
    assert!(md.contains("## Dispatch Resolution"));
    assert!(md.contains("`Foo`[cp42]"));
    assert!(md.contains("## Native Boundary"));
    assert!(md.contains("`java/lang/String.intern`"));
}

#[test]
fn test_telemetry_markdown_uncaught() {
    let mut store = TelemetryStore::new();
    store
        .exception_flow
        .record_throw("java/lang/Exception", "Foo", "bar", 10);

    let md = store.to_markdown_report();
    assert!(md.contains("`uncaught`"));
}

#[test]
fn test_telemetry_markdown_empty() {
    let store = TelemetryStore::new();
    let md = store.to_markdown_report();

    assert!(md.contains("No class initialization events recorded."));
    assert!(md.contains("No exception flow events recorded."));
}

#[test]
fn test_telemetry_new() {
    let store = TelemetryStore::new();
    assert!(store.class_init_dag.events.is_empty());
}

#[test]
#[cfg(feature = "telemetry")]
fn test_telemetry_json() {
    let store = TelemetryStore::new();
    let json = store.to_json();
    assert!(json.contains('{'));
}
