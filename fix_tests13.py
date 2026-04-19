import sys

with open('crates/duke-telemetry/src/lib.rs', 'r') as f:
    content = f.read()

import re

test_code = """
    #[test]
    fn test_to_json() {
        let mut store = crate::TelemetryStore::default();
        store.bytecode_cost.record("iadd", "Foo", "bar", 10, 100);
        let json = store.to_json();
        assert!(json.contains("iadd"));
        assert!(json.contains("bytecode_cost"));
    }
"""

last_brace = content.rfind('}')
content = content[:last_brace] + test_code + "\n}"


with open('crates/duke-telemetry/src/lib.rs', 'w') as f:
    f.write(content)
