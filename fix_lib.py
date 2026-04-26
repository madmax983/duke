with open("crates/duke-telemetry/src/lib.rs", "r") as f:
    lib = f.read()

test_code = """

    #[test]
    #[cfg(feature = "telemetry")]
    fn test_print_report_io_error() {
        struct FailingWriter;
        impl std::io::Write for FailingWriter {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disk full"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store.object_lineage.record("java/lang/String", "Foo", 10, "bar");
        store.class_init_dag.record("java/lang/String", "java/lang/System", 500);
        store.exception_flow.record_throw("java/lang/Exception", "Foo", "bar", 10);
        store.dispatch_resolution.record("Foo", 42, "java/lang/String", true);
        store.native_boundary.record_call("java/lang/String", "intern", 100, true);

        let mut w = FailingWriter;
        let res = store.print_report(&mut w);
        assert!(res.is_err());
    }
"""

lib = lib.replace("""

    struct FailingWriter;
    impl std::io::Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("disk full"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    #[cfg(feature = "telemetry")]
    fn test_print_report_io_error() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store.object_lineage.record("java/lang/String", "Foo", 10, "bar");
        store.class_init_dag.record("java/lang/String", "java/lang/System", 500);
        store.exception_flow.record_throw("java/lang/Exception", "Foo", "bar", 10);
        store.dispatch_resolution.record("Foo", 42, "java/lang/String", true);
        store.native_boundary.record_call("java/lang/String", "intern", 100, true);

        let mut w = FailingWriter;
        let res = store.print_report(&mut w);
        assert!(res.is_err());
    }
""", test_code)

with open("crates/duke-telemetry/src/lib.rs", "w") as f:
    f.write(lib)
