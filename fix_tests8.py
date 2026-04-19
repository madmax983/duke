import sys

with open('crates/duke-telemetry/src/lib.rs', 'r') as f:
    content = f.read()

import re

content = content.replace("#[test]\n    fn test_to_markdown_report", "#[test]\n        fn test_to_markdown_report")

with open('crates/duke-telemetry/src/lib.rs', 'w') as f:
    f.write(content)
