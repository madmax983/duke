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
