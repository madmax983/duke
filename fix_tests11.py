import sys

with open('crates/duke-telemetry/src/lib.rs', 'r') as f:
    content = f.read()

import re

test_code = """
    #[test]
    fn test_print_report_empty() {
        let mut store = crate::TelemetryStore::default();
        let mut buf = Vec::new();
        store.print_report(&mut buf).unwrap();
        let report = String::from_utf8(buf).unwrap();
        assert!(report.contains("=== Duke VM Telemetry Report ==="));
    }

    #[test]
    fn test_to_markdown_report_empty() {
        let mut store = crate::TelemetryStore::default();
        let md = store.to_markdown_report();
        assert!(md.contains("# Duke VM Telemetry Report"));
    }
"""

last_brace = content.rfind('}')
content = content[:last_brace] + test_code + "\n}"


with open('crates/duke-telemetry/src/lib.rs', 'w') as f:
    f.write(content)
