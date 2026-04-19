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
        store.object_lineage.record("com/Example", "java/lang/Object", 1);
        store.class_init_dag.record("java/lang/String", "java/lang/System", 500);
        store.exception_flow.record("java/lang/Exception", "ThrowSite", Some(("CatchSite", 10)));
        store.dispatch_resolution.record("Caller", "java/lang/Object", 1, 1);
        store.native_boundary.record("java/lang/System.out", false, 500);
        let mut buf = Vec::new();
        store.print_report(&mut buf).unwrap();
        let report = String::from_utf8(buf).unwrap();
        assert!(report.contains("=== Duke VM Telemetry Report ==="));
        assert!(report.contains("iadd"));
        assert!(report.contains("com/Example"));
        assert!(report.contains("java/lang/String"));
        assert!(report.contains("java/lang/Exception"));
        assert!(report.contains("Caller"));
        assert!(report.contains("java/lang/System.out"));
    }
}
"""

new_content = content[:last_brace] + test_code

with open('crates/duke-telemetry/src/lib.rs', 'w') as f:
    f.write(new_content)
