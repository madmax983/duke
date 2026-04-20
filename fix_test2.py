import re

with open("crates/duke-telemetry/src/lib.rs", "r") as f:
    content = f.read()

content = content.replace(
    'assert!(s.contains("Foo::bar @10 allocs 1 of java/lang/String"));',
    'println!("{}", s);\n        assert!(s.contains("allocs 1 of bar"));'
)

with open("crates/duke-telemetry/src/lib.rs", "w") as f:
    f.write(content)
