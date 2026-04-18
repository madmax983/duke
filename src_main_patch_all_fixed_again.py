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
