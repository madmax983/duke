import re
import sys

content = sys.stdin.read()

# fix clippy::map_unwrap_or
content = re.sub(
    r"\.map\(\|o\| o\.fields\[1 \+ i \* 2\]\)\n\s*\.unwrap_or\(Slot::Reference\(None\)\)",
    ".map_or(Slot::Reference(None), |o| o.fields[1 + i * 2])",
    content
)

content = re.sub(
    r"\.map\(\|o\| o\.fields\[2 \+ i \* 2\]\)\n\s*\.unwrap_or\(Slot::Reference\(None\)\)",
    ".map_or(Slot::Reference(None), |o| o.fields[2 + i * 2])",
    content
)

content = re.sub(
    r"\.map\(\|c\| c\.instance_field_count\)\n\s*\.unwrap_or\(0\)",
    ".map_or(0, |c| c.instance_field_count)",
    content
)

print(content)
