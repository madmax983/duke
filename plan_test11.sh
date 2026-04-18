cat << 'INNER_EOF' > src_main_patch_duke.py
import re
import sys

content = sys.stdin.read()

# fix clippy::unnecessary_sort_by in duke/src/jar_analyze.rs
content = re.sub(
    r"all_methods\.sort_by\(\|a, b\| b\.complexity\.cmp\(&a\.complexity\)\);",
    "all_methods.sort_by_key(|b| std::cmp::Reverse(b.complexity));",
    content
)

print(content)
INNER_EOF
cat duke/src/jar_analyze.rs | python3 src_main_patch_duke.py > jar_analyze_patched.rs
cp jar_analyze_patched.rs duke/src/jar_analyze.rs

cargo clippy --all-targets --all-features -- -D warnings
