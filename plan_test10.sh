cat << 'INNER_EOF' > src_main_patch_all_fixed_again.py
import re
import sys

content = sys.stdin.read()

# fix clippy::unnecessary_sort_by
content = re.sub(
    r"ops\.sort_by\(\|a, b\| b\.1\.count\.cmp\(&a\.1\.count\)\);",
    "ops.sort_by_key(|b| std::cmp::Reverse(b.1.count));",
    content
)
content = re.sub(
    r"sites\.sort_by\(\|a, b\| b\.1\.count\.cmp\(&a\.1\.count\)\);",
    "sites.sort_by_key(|b| std::cmp::Reverse(b.1.count));",
    content
)
content = re.sub(
    r"dsites\.sort_by\(\|a, b\| b\.1\.calls\.cmp\(&a\.1\.calls\)\);",
    "dsites.sort_by_key(|b| std::cmp::Reverse(b.1.calls));",
    content
)
content = re.sub(
    r"natives\.sort_by\(\|a, b\| b\.1\.calls\.cmp\(&a\.1\.calls\)\);",
    "natives.sort_by_key(|b| std::cmp::Reverse(b.1.calls));",
    content
)

print(content)
INNER_EOF
cat crates/duke-telemetry/src/lib.rs | python3 src_main_patch_all_fixed_again.py > telemetry_patched.rs
cp telemetry_patched.rs crates/duke-telemetry/src/lib.rs

cat << 'INNER_EOF' > src_main_patch_native_again.py
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
INNER_EOF
cat crates/duke-interpreter/src/native.rs | python3 src_main_patch_native_again.py > native_patched_again.rs
cp native_patched_again.rs crates/duke-interpreter/src/native.rs

cargo clippy --all-targets --all-features -- -D warnings
