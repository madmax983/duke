use duke_telemetry::TelemetryStore;
use std::io::{self, Write};

struct FailingWriter {
    calls: usize,
    fail_on_call: usize,
}

impl FailingWriter {
    fn new(fail_on_call: usize) -> Self {
        Self {
            calls: 0,
            fail_on_call,
        }
    }
}

impl Write for FailingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        if self.calls == self.fail_on_call {
            Err(io::Error::other("mock error"))
        } else {
            Ok(buf.len())
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.calls == self.fail_on_call {
            Err(io::Error::other("mock error flush"))
        } else {
            Ok(())
        }
    }
}

#[test]
fn test_print_bytecode_cost_io_error_inner() {
    let mut store = TelemetryStore::default();
    store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
    let mut w = FailingWriter::new(2);
    assert!(store.print_report(&mut w).is_err());
}

#[test]
fn test_print_object_lineage_io_error_inner() {
    let mut store = TelemetryStore::default();
    store.object_lineage.record("java/lang/String", "Foo", 10, "bar");
    let mut w = FailingWriter::new(2);
    assert!(store.print_report(&mut w).is_err());
}

#[test]
fn test_print_class_init_dag_io_error_inner() {
    let mut store = TelemetryStore::default();
    store.class_init_dag.record("java/lang/String", "java/lang/System", 500);
    let mut w = FailingWriter::new(2);
    assert!(store.print_report(&mut w).is_err());
}

#[test]
fn test_print_exception_flow_io_error_inner() {
    let mut store = TelemetryStore::default();
    store.exception_flow.record_throw("java/lang/Exception", "Foo", "bar", 10);
    let mut w = FailingWriter::new(2);
    assert!(store.print_report(&mut w).is_err());
}

#[test]
fn test_print_dispatch_resolution_io_error_inner() {
    let mut store = TelemetryStore::default();
    store.dispatch_resolution.record("Foo", 42, "java/lang/String", true);
    let mut w = FailingWriter::new(2);
    assert!(store.print_report(&mut w).is_err());
}

#[test]
fn test_print_native_boundary_io_error_inner() {
    let mut store = TelemetryStore::default();
    store.native_boundary.record_call("java/lang/String", "intern", 100, true);
    let mut w = FailingWriter::new(2);
    assert!(store.print_report(&mut w).is_err());
}

#[test]
fn test_print_dispatch_resolution_populated() {
    let mut store = TelemetryStore::default();
    store.dispatch_resolution.record("Foo", 42, "java/lang/String", true);
    let mut buf = Vec::new();
    store.print_report(&mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("Foo"));
}

#[test]
fn test_print_native_boundary_populated() {
    let mut store = TelemetryStore::default();
    store.native_boundary.record_call("java/lang/String", "intern", 100, true);
    let mut buf = Vec::new();
    store.print_report(&mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("intern"));
}

#[test]
fn test_markdown_exception_flow_caught() {
    let mut store = TelemetryStore::default();
    store.exception_flow.record_throw("java/lang/Exception", "Foo", "bar", 10);
    store.exception_flow.record_catch(0, "Foo", "bar", 20);
    let s = store.to_markdown_report();
    assert!(s.contains("Foo::bar @20"));
}

#[test]
fn test_markdown_dispatch_resolution() {
    let mut store = TelemetryStore::default();
    store.dispatch_resolution.record("Foo", 42, "java/lang/String", true);
    let s = store.to_markdown_report();
    assert!(s.contains("Foo"));
}

#[test]
fn test_markdown_native_boundary() {
    let mut store = TelemetryStore::default();
    store.native_boundary.record_call("java/lang/String", "intern", 100, true);
    let s = store.to_markdown_report();
    assert!(s.contains("intern"));
}

#[test]
fn test_markdown_object_lineage() {
    let mut store = TelemetryStore::default();
    store.object_lineage.record("java/lang/String", "Foo", 10, "bar");
    let s = store.to_markdown_report();
    assert!(s.contains("java/lang/String"));
}

#[test]
fn test_markdown_class_init_dag_empty() {
    let store = TelemetryStore::default();
    let s = store.to_markdown_report();
    assert!(s.contains("No class initialization events recorded."));
}

#[test]
fn test_markdown_exception_flow_empty() {
    let store = TelemetryStore::default();
    let s = store.to_markdown_report();
    assert!(s.contains("No exception flow events recorded."));
}

#[test]
fn test_print_class_init_dag_io_error_inner_2() {
    let mut store = TelemetryStore::default();
    store.class_init_dag.record("java/lang/String", "java/lang/System", 500);
    store.class_init_dag.record("java/lang/String2", "java/lang/System2", 500);
    let mut w = FailingWriter::new(4);
    assert!(store.print_report(&mut w).is_err());
}

#[test]
fn test_print_exception_flow_io_error_inner_2() {
    let mut store = TelemetryStore::default();
    store.exception_flow.record_throw("java/lang/Exception", "Foo", "bar", 10);
    store.exception_flow.record_throw("java/lang/Exception2", "Foo", "bar", 10);
    let mut w = FailingWriter::new(4);
    assert!(store.print_report(&mut w).is_err());
}

#[test]
fn test_print_bytecode_cost_io_error_inner_2() {
    let mut store = TelemetryStore::default();
    store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
    store.bytecode_cost.record("isub", "Foo", "bar", 10, 100);
    let mut w = FailingWriter::new(4);
    assert!(store.print_report(&mut w).is_err());
}

#[test]
fn test_print_object_lineage_io_error_inner_2() {
    let mut store = TelemetryStore::default();
    store.object_lineage.record("java/lang/String", "Foo", 10, "bar");
    store.object_lineage.record("java/lang/String2", "Foo", 10, "bar");
    let mut w = FailingWriter::new(4);
    assert!(store.print_report(&mut w).is_err());
}

#[test]
fn test_print_dispatch_resolution_io_error_inner_2() {
    let mut store = TelemetryStore::default();
    store.dispatch_resolution.record("Foo", 42, "java/lang/String", true);
    store.dispatch_resolution.record("Foo2", 42, "java/lang/String", true);
    let mut w = FailingWriter::new(4);
    assert!(store.print_report(&mut w).is_err());
}

#[test]
fn test_print_native_boundary_io_error_inner_2() {
    let mut store = TelemetryStore::default();
    store.native_boundary.record_call("java/lang/String", "intern", 100, true);
    store.native_boundary.record_call("java/lang/String2", "intern", 100, true);
    let mut w = FailingWriter::new(4);
    assert!(store.print_report(&mut w).is_err());
}
