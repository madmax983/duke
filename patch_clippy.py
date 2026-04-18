import re

with open("crates/duke-interpreter/src/native.rs", "r") as f:
    content = f.read()

content = content.replace(
    ".map(|o| o.fields[1 + i * 2])\n                .unwrap_or(Slot::Reference(None))",
    ".map_or(Slot::Reference(None), |o| o.fields[1 + i * 2])"
)
content = content.replace(
    ".map(|o| o.fields[2 + i * 2])\n                .unwrap_or(Slot::Reference(None))",
    ".map_or(Slot::Reference(None), |o| o.fields[2 + i * 2])"
)
content = content.replace(
    ".map(|o| o.fields[1 + i * 2])\n                .unwrap_or(Slot::Reference(None));",
    ".map_or(Slot::Reference(None), |o| o.fields[1 + i * 2]);"
)
content = content.replace(
    ".map(|o| o.fields[2 + i * 2])\n                .unwrap_or(Slot::Reference(None));",
    ".map_or(Slot::Reference(None), |o| o.fields[2 + i * 2]);"
)
content = content.replace(
    ".map(|c| c.instance_field_count)\n        .unwrap_or(0);",
    ".map_or(0, |c| c.instance_field_count);"
)
with open("crates/duke-interpreter/src/native.rs", "w") as f:
    f.write(content)
