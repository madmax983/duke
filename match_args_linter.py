import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    text = f.read()

count = 0
for match in re.finditer(r'let\s+(\w+)\s*=\s*match\s+args\.get\(\s*(\d+)\s*\)\s*\{([^}]*TypeMismatch[^}]*)\};', text, re.DOTALL):
    var = match.group(1)
    idx = match.group(2)
    body = match.group(3)
    full_match = match.group(0)

    if "Some(Slot::Int" in body:
        print(f"Match block at args.get({idx}) for {var} returns Err:")
        print(match.group(0))
        print("-" * 40)
        count += 1
    elif "Some(Slot::Double" in body:
        print(f"Match block at args.get({idx}) for {var} returns Err:")
        print(match.group(0))
        print("-" * 40)
        count += 1
    elif "Some(Slot::Long" in body:
        print(f"Match block at args.get({idx}) for {var} returns Err:")
        print(match.group(0))
        print("-" * 40)
        count += 1
    elif "Some(Slot::Float" in body:
        print(f"Match block at args.get({idx}) for {var} returns Err:")
        print(match.group(0))
        print("-" * 40)
        count += 1

print(f"Total found: {count}")
