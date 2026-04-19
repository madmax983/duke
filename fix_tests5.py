import sys

with open('crates/duke-telemetry/src/lib.rs', 'r') as f:
    content = f.read()

import re
# modify telemetry_store_to_markdown_report to add records for other stores too
old_test = """    fn telemetry_store_to_markdown_report() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store
            .class_init_dag
            .record("java/lang/String", "java/lang/System", 500);

        let md = store.to_markdown_report();
        assert!(md.contains("# Duke VM Telemetry Report"));
        assert!(md.contains("| `iadd` | 1 | 100 |"));
        assert!(md.contains("```mermaid\\n"));
        assert!(md.contains("graph TD;\\n"));
        assert!(md.contains("\\"java/lang/System\\" -->|500ns| \\"java/lang/String\\";"));
        assert!(md.contains("```"));
    }"""

new_test = """    fn telemetry_store_to_markdown_report() {
        let mut store = TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        store
            .class_init_dag
            .record("java/lang/String", "java/lang/System", 500);
        store.object_lineage.record("com/Example", "method", 1, "java/lang/Object");
        let ev = store.exception_flow.record_throw("java/lang/Exception", "ThrowClass", "ThrowMethod", 1);
        store.exception_flow.record_catch(ev, "CatchClass", "CatchMethod", 2);
        store.dispatch_resolution.record("CallerClass", 1, "CallerMethod", false);
        store.native_boundary.record_call("java/lang/System", "out", 500, false);

        let md = store.to_markdown_report();
        assert!(md.contains("# Duke VM Telemetry Report"));
        assert!(md.contains("| `iadd` | 1 | 100 |"));
        assert!(md.contains("```mermaid\\n"));
        assert!(md.contains("graph TD;\\n"));
        assert!(md.contains("\\"java/lang/System\\" -->|500ns| \\"java/lang/String\\";"));
        assert!(md.contains("```"));
        assert!(md.contains("com/Example::method @1"));
        assert!(md.contains("java/lang/Exception"));
        assert!(md.contains("CallerClass::CallerMethod"));
        assert!(md.contains("java/lang/System::out"));
    }"""

content = content.replace(old_test, new_test)

with open('crates/duke-telemetry/src/lib.rs', 'w') as f:
    f.write(content)
