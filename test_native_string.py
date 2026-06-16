import re

with open('crates/duke-interpreter/src/native.rs', 'r') as f:
    lines = f.readlines()

count = 0
for i, line in enumerate(lines):
    if "string_value.clone().unwrap_or_default()" in line:
        count += 1
print(f"Found {count} instances of string_value.clone().unwrap_or_default()")
