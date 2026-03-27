import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    text = f.read()

count = 0
for match in re.finditer(r'let\s+a\s*=\s*match\s+args\.get\(\s*(\d+)\s*\)\s*\{([^}]*)\};', text, re.DOTALL):
    var = match.group(1)
    idx = match.group(2)
    body = match.group(2)

    if "NullPointerException" in body:
        print(f"Match block at args.get({idx}) for a returns Err:")
        print(match.group(0))
        print("-" * 40)
        count += 1

print(f"Total found: {count}")
