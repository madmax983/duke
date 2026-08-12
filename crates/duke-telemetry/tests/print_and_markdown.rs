//! Telemetry store testing module
//!
//! This module verifies the string formatting logic of the telemetry subsystem.
//! It ensures that when execution metadata is exported (either to standard output
//! or as a Markdown report), the structural integrity of the output is maintained.
//!
//! These integration tests act as safeguards against unintended regressions in the
//! telemetry reporting interface, confirming that top-level headers (like
//! `Bytecode Cost (Top 10)`) are accurately rendered for developers to analyze.

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
fn test_markdown_object_lineage_empty() {
    let store = TelemetryStore::new();
    let output = store.to_markdown_report();
    assert!(output.contains("## Object Lineage (Top 10 Allocation Sites)"));
}

#[test]
fn test_markdown_class_init_dag_empty() {
    let store = TelemetryStore::new();
    let output = store.to_markdown_report();
    assert!(output.contains("## Class Initialization DAG"));
    assert!(output.contains("No class initialization events recorded."));
}

#[test]
fn test_markdown_exception_flow_empty() {
    let store = TelemetryStore::new();
    let output = store.to_markdown_report();
    assert!(output.contains("## Exception Flow"));
    assert!(output.contains("No exception flow events recorded."));
}

#[test]
fn test_markdown_dispatch_resolution_empty() {
    let store = TelemetryStore::new();
    let output = store.to_markdown_report();
    assert!(output.contains("## Dispatch Resolution (Top 10 Virtual Call Sites)"));
}

#[test]
fn test_markdown_native_boundary_empty() {
    let store = TelemetryStore::new();
    let output = store.to_markdown_report();
    assert!(output.contains("## Native Boundary (Top 10 by Call Count)"));
}

#[test]
fn test_markdown_bytecode_cost_populated() {
    let mut store = TelemetryStore::new();
    store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
    let output = store.to_markdown_report();
    assert!(output.contains("## Bytecode Cost (Top 10)"));
    assert!(output.contains("`iadd`"));
}

#[test]
fn test_markdown_object_lineage_populated() {
    let mut store = TelemetryStore::new();
    store
        .object_lineage
        .record("java/lang/String", "Foo", 10, "bar");
    let output = store.to_markdown_report();
    assert!(output.contains("## Object Lineage (Top 10 Allocation Sites)"));
    assert!(output.contains("java/lang/String"));
}

#[test]
fn test_markdown_class_init_dag_populated() {
    let mut store = TelemetryStore::new();
    store
        .class_init_dag
        .record("java/lang/String", "java/lang/System", 500);
    let output = store.to_markdown_report();
    assert!(output.contains("## Class Initialization DAG"));
    assert!(output.contains("java/lang/String"));
    assert!(output.contains("```mermaid"));
}

#[test]
fn test_markdown_exception_flow_populated() {
    let mut store = TelemetryStore::new();
    let event_idx = store
        .exception_flow
        .record_throw("java/lang/Exception", "Foo", "bar", 10);
    let output = store.to_markdown_report();
    assert!(output.contains("## Exception Flow"));
    assert!(output.contains("java/lang/Exception"));
    assert!(output.contains("uncaught"));

    store
        .exception_flow
        .record_catch(event_idx, "Foo", "baz", 20);
    let output_caught = store.to_markdown_report();
    assert!(output_caught.contains("Foo::baz @20"));
}

#[test]
fn test_markdown_dispatch_resolution_populated() {
    let mut store = TelemetryStore::new();
    store
        .dispatch_resolution
        .record("java/lang/String", 10, "java/lang/String", false);
    let output = store.to_markdown_report();
    assert!(output.contains("## Dispatch Resolution (Top 10 Virtual Call Sites)"));
    assert!(output.contains("java/lang/String"));
}

#[test]
fn test_markdown_native_boundary_populated() {
    let mut store = TelemetryStore::new();
    store
        .native_boundary
        .record_call("java/lang/System", "currentTimeMillis", 1000, false);
    let output = store.to_markdown_report();
    assert!(output.contains("## Native Boundary (Top 10 by Call Count)"));
    assert!(output.contains("java/lang/System.currentTimeMillis"));
}
