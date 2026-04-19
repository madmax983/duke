import sys

with open('crates/duke-telemetry/src/lib.rs', 'r') as f:
    content = f.read()

import re

# replace mut store with just store in the empty tests
content = content.replace("let mut store = crate::TelemetryStore::default();\n        let mut buf = Vec::new();\n        store.print_report(&mut buf).unwrap();", "let store = crate::TelemetryStore::default();\n        let mut buf = Vec::new();\n        store.print_report(&mut buf).unwrap();")
content = content.replace("let mut store = crate::TelemetryStore::default();\n        let md = store.to_markdown_report();", "let store = crate::TelemetryStore::default();\n        let md = store.to_markdown_report();")


with open('crates/duke-telemetry/src/lib.rs', 'w') as f:
    f.write(content)
