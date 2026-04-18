import re

file_path = "crates/duke-telemetry/src/lib.rs"
with open(file_path, "r") as f:
    content = f.read()

# ops.sort_by(|a, b| b.1.count.cmp(&a.1.count))
content = re.sub(
    r'ops\.sort_by\(\|a, b\| b\.1\.count\.cmp\(&a\.1\.count\)\);',
    r'ops.sort_by_key(|b| std::cmp::Reverse(b.1.count));',
    content
)

# sites.sort_by(|a, b| b.1.count.cmp(&a.1.count))
content = re.sub(
    r'sites\.sort_by\(\|a, b\| b\.1\.count\.cmp\(&a\.1\.count\)\);',
    r'sites.sort_by_key(|b| std::cmp::Reverse(b.1.count));',
    content
)

# dsites.sort_by(|a, b| b.1.calls.cmp(&a.1.calls))
content = re.sub(
    r'dsites\.sort_by\(\|a, b\| b\.1\.calls\.cmp\(&a\.1\.calls\)\);',
    r'dsites.sort_by_key(|b| std::cmp::Reverse(b.1.calls));',
    content
)

# natives.sort_by(|a, b| b.1.calls.cmp(&a.1.calls))
content = re.sub(
    r'natives\.sort_by\(\|a, b\| b\.1\.calls\.cmp\(&a\.1\.calls\)\);',
    r'natives.sort_by_key(|b| std::cmp::Reverse(b.1.calls));',
    content
)

with open(file_path, "w") as f:
    f.write(content)
