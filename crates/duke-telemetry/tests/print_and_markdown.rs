//! Integration tests for `TelemetryStore`'s markdown and formatting capabilities.

use duke_telemetry::TelemetryStore;

#[test]
fn test_print_bytecode_cost_multiple() {
    let mut store = TelemetryStore::default();
    let static_ops: Vec<&'static str> = (0..15).map(|i| format!("op{i}")).map(|s| Box::leak(s.into_boxed_str()) as &'static str).collect();

    for (i, op) in static_ops.iter().enumerate().take(15) {
        // give each operation a different count so the order is deterministic
        store.bytecode_cost.record(
            op,
            "Foo",
            "bar",
            10,
            100,
        );
        // let's record it multiple times to ensure count differences
        for _ in 0..i {
            store.bytecode_cost.record(
                op,
                "Foo",
                "bar",
                10,
                100,
            );
        }
    }
    let mut buf = Vec::new();
    store.print_report(&mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("=== Duke VM Telemetry Report ==="));
    assert!(s.contains("-- bytecode_cost (top 10 by count) --"));
    assert!(s.contains("op14"));
    assert!(!s.contains("op0")); // Top 10 only
}

#[test]
fn test_print_object_lineage_multiple() {
    let mut store = TelemetryStore::default();
    for i in 0..15 {
        store
            .object_lineage
            .record("java/lang/String", "Foo", i, "bar");
        for _ in 0..i {
            store
                .object_lineage
                .record("java/lang/String", "Foo", i, "bar");
        }
    }
    let mut buf = Vec::new();
    store.print_report(&mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("java/lang/String"));
}

#[test]
fn test_print_class_init_dag_empty() {
    let store = TelemetryStore::default();
    let mut buf = Vec::new();
    store.print_report(&mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("No class initialization events recorded."));
}

#[test]
fn test_markdown_methods_empty() {
    let store = TelemetryStore::default();
    let md = store.to_markdown_report();
    assert!(md.contains("No class initialization events recorded."));
    assert!(md.contains("No exception flow events recorded."));
}

#[test]
#[allow(clippy::cast_possible_truncation)]
fn test_markdown_methods_multiple() {
    let mut store = TelemetryStore::default();

    // Leak the strings once so we can use them as static
    let static_ops: Vec<&'static str> = (0..15).map(|i| format!("op{i}")).map(|s| Box::leak(s.into_boxed_str()) as &'static str).collect();
    let static_methods: Vec<&'static str> = (0..15).map(|i| format!("m{i}")).map(|s| Box::leak(s.into_boxed_str()) as &'static str).collect();
    let static_natives: Vec<&'static str> = (0..15).map(|i| format!("n{i}")).map(|s| Box::leak(s.into_boxed_str()) as &'static str).collect();

    for i in 0..15 {
        for _ in 0..i {
            store.bytecode_cost.record(
                static_ops[i],
                "Foo",
                "bar",
                10,
                100,
            );
            store.object_lineage.record(
                "java/lang/String",
                static_methods[i],
                i,
                "bar",
            );
            store
                .dispatch_resolution
                .record("Foo", i as u16, "java/lang/String", true);
            store.native_boundary.record_call(
                "java/lang/String",
                static_natives[i],
                100,
                true,
            );
        }
    }
    let md = store.to_markdown_report();
    assert!(md.contains("op14"));
    assert!(md.contains("m14"));
    assert!(md.contains("cp14"));
    assert!(md.contains("n14"));
}
