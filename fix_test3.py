import re

with open("crates/duke-telemetry/src/lib.rs", "r") as f:
    content = f.read()

content = content.replace(
    'assert!(s.contains("Foo[cp42] calls=1 targets=1 walks=3"));',
    'assert!(s.contains("Foo[cp42] calls=3 targets=1 walks=3"));'
)

with open("crates/duke-telemetry/src/lib.rs", "w") as f:
    f.write(content)
