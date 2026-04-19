import sys

with open('crates/duke-telemetry/src/lib.rs', 'r') as f:
    content = f.read()

import re

test_code = """
    #[test]
    fn test_to_markdown_report() {
        let mut store = crate::TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store.object_lineage.record("com/Example", "method", 1, "java/lang/Object");
        store.class_init_dag.record("java/lang/String", "java/lang/System", 500);
        let ev = store.exception_flow.record_throw("java/lang/Exception", "ThrowClass", "ThrowMethod", 1);
        store.exception_flow.record_catch(ev, "CatchClass", "CatchMethod", 2);
        store.dispatch_resolution.record("CallerClass", 1, "CallerMethod", false);
        store.native_boundary.record_call("java/lang/System", "out", 500, false);
        let md = store.to_markdown_report();
        assert!(md.contains("# Duke VM Telemetry Report"));
        assert!(md.contains("iadd"));
        assert!(md.contains("com/Example"));
        assert!(md.contains("java/lang/String"));
        assert!(md.contains("java/lang/Exception"));
        assert!(md.contains("CallerClass"));
        assert!(md.contains("java/lang/System"));
    }
"""

last_brace = content.rfind('}')
content = content[:last_brace] + test_code + "\n}"

with open('crates/duke-telemetry/src/lib.rs', 'w') as f:
    f.write(content)
