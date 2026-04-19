import sys

with open('crates/duke-telemetry/src/lib.rs', 'r') as f:
    content = f.read()

import re

# replace test_to_markdown_report and test_print_report to be inside mod tests
content = re.sub(r'#\[test\]\n\s*fn test_to_markdown_report.*?\n\s*}', '', content, flags=re.DOTALL)
content = re.sub(r'#\[test\]\n\s*fn test_print_report.*?\n\s*}', '', content, flags=re.DOTALL)

last_brace = content.rfind('}')

test_code = """
    #[test]
    fn test_print_report() {
        let mut store = crate::TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store.object_lineage.record("com/Example", "method", 1, "java/lang/Object");
        store.class_init_dag.record("java/lang/String", "java/lang/System", 500);
        let ev = store.exception_flow.record_throw("java/lang/Exception", "ThrowClass", "ThrowMethod", 1);
        store.exception_flow.record_catch(ev, "CatchClass", "CatchMethod", 2);
        store.dispatch_resolution.record("CallerClass", 1, "CallerMethod", false);
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
"""
content = content[:last_brace] + test_code + "\n}"


with open('crates/duke-telemetry/src/lib.rs', 'w') as f:
    f.write(content)
