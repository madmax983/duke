import re

with open("crates/duke-telemetry/src/lib.rs", "r") as f:
    content = f.read()

content = content.replace(
    'assert!(md.contains("graph TD;\\n"));',
    'assert!(md.contains("graph TD;\\n"));\n\n        // Test formatting of class without initialization events\n        let empty_store = TelemetryStore::default();\n        let empty_md = empty_store.to_markdown_report();\n        assert!(empty_md.contains("No class initialization events recorded."));\n'
)

with open("crates/duke-telemetry/src/lib.rs", "w") as f:
    f.write(content)
