import sys

with open('crates/duke-telemetry/src/lib.rs', 'r') as f:
    content = f.read()

# remove old test
import re
content = re.sub(r'#\[test\]\s*#\[cfg\(feature = "telemetry"\)\]\s*fn telemetry_store_print_report\(\) \{.*?\n    }', '', content, flags=re.DOTALL)


last_brace = content.rfind('}')

test_code = """
    #[test]
    #[cfg(feature = "telemetry")]
    fn telemetry_store_print_report() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store.object_lineage.record("com/Example", "method", 1, "java/lang/Object");
        store.class_init_dag.record("java/lang/String", "java/lang/System", 500);
        let ev = store.exception_flow.record_throw("java/lang/Exception", "ThrowClass", "ThrowMethod", 1);
        store.exception_flow.record_catch(ev, "CatchClass", "CatchMethod", 2);
        store.dispatch_resolution.record("CallerClass", "CallerMethod", 1, "TargetClass", "TargetMethod", false);
        store.native_boundary.record_call("java/lang/System", "out", 500, false);
        let mut buf = Vec::new();
        store.print_report(&mut buf).unwrap();
        let report = String::from_utf8(buf).unwrap();
        assert!(report.contains("=== Duke VM Telemetry Report ==="));
        assert!(report.contains("iadd"));
        assert!(report.contains("com/Example"));
        assert!(report.contains("java/lang/String"));
        assert!(report.contains("java/lang/Exception"));
        assert!(report.contains("CallerClass"));
        assert!(report.contains("java/lang/System"));
    }
}
"""

new_content = content[:last_brace] + test_code

with open('crates/duke-telemetry/src/lib.rs', 'w') as f:
    f.write(new_content)
