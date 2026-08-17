use duke_telemetry::TelemetryStore;
use std::io::{Error, Result, Write};

struct FailingWriter {
    max_writes: usize,
    writes: usize,
}

impl Write for FailingWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if self.writes >= self.max_writes {
            Err(Error::other(format!(
                "write limit {} reached",
                self.max_writes
            )))
        } else {
            self.writes += 1;
            Ok(buf.len())
        }
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

#[test]
fn test_print_report_io_error_propagates() {
    let mut store = TelemetryStore::default();

    // Test that even with empty/unpopulated states, errors propagate
    let mut empty_max_writes = 0;
    loop {
        let mut w = FailingWriter {
            max_writes: empty_max_writes,
            writes: 0,
        };
        let res = store.print_report(&mut w);
        if res.is_ok() {
            break;
        }
        empty_max_writes += 1;
    }

    // Fully populate it so all paths are active
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
        .record("java/lang/String", 42, "java/lang/String", true);
    store
        .native_boundary
        .record_call("java/lang/String", "intern", 100, true);

    let mut full_max_writes = 0;
    loop {
        let mut w = FailingWriter {
            max_writes: full_max_writes,
            writes: 0,
        };
        let res = store.print_report(&mut w);
        if res.is_ok() {
            break;
        }
        full_max_writes += 1;
    }

    // We expect formatting the full state to take significantly more writes
    assert!(full_max_writes > empty_max_writes);
}
