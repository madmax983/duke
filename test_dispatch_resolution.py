import sys

with open('crates/duke-telemetry/src/lib.rs', 'r') as f:
    content = f.read()

import re

# To increase coverage for lib.rs we might need to test more edge cases.
# Actually let's check what lines are untested in duke-telemetry
