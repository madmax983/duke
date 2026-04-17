import re

with open('crates/duke-telemetry/src/lib.rs', 'r') as f:
    content = f.read()

content = re.sub(
    r'ops\.sort_by\(\|a, b\| b\.1\.count\.cmp\(&a\.1\.count\)\);',
    'ops.sort_by_key(|b| std::cmp::Reverse(b.1.count));',
    content
)

content = re.sub(
    r'sites\.sort_by\(\|a, b\| b\.1\.count\.cmp\(&a\.1\.count\)\);',
    'sites.sort_by_key(|b| std::cmp::Reverse(b.1.count));',
    content
)

content = re.sub(
    r'dsites\.sort_by\(\|a, b\| b\.1\.calls\.cmp\(&a\.1\.calls\)\);',
    'dsites.sort_by_key(|b| std::cmp::Reverse(b.1.calls));',
    content
)

content = re.sub(
    r'natives\.sort_by\(\|a, b\| b\.1\.calls\.cmp\(&a\.1\.calls\)\);',
    'natives.sort_by_key(|b| std::cmp::Reverse(b.1.calls));',
    content
)

with open('crates/duke-telemetry/src/lib.rs', 'w') as f:
    f.write(content)

with open('crates/duke-interpreter/src/native.rs', 'r') as f:
    content = f.read()

content = re.sub(
    r'\.map\(\|o\| o\.fields\[1 \+ i \* 2\]\)\n\s*\.unwrap_or\(Slot::Reference\(None\)\)',
    '.map_or(Slot::Reference(None), |o| o.fields[1 + i * 2])',
    content
)

content = re.sub(
    r'\.map\(\|o\| o\.fields\[2 \+ i \* 2\]\)\n\s*\.unwrap_or\(Slot::Reference\(None\)\)',
    '.map_or(Slot::Reference(None), |o| o.fields[2 + i * 2])',
    content
)

content = re.sub(
    r'\.map\(\|c\| c\.instance_field_count\)\n\s*\.unwrap_or\(0\)',
    '.map_or(0, |c| c.instance_field_count)',
    content
)


with open('crates/duke-interpreter/src/native.rs', 'w') as f:
    f.write(content)
