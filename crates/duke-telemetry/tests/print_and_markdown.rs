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
#[cfg(feature = "telemetry")]
fn test_print_bytecode_cost_empty() {
    let store = TelemetryStore::default();
    let mut buf = Vec::new();
    store.print_report(&mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("-- bytecode_cost (top 10 by count) --"));
}

#[test]
#[cfg(feature = "telemetry")]
fn test_markdown_bytecode_cost_empty() {
    let store = TelemetryStore::default();
    let output = store.to_markdown_report();
    assert!(output.contains("## Bytecode Cost (Top 10)"));
}
