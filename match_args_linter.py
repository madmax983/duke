import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    code = f.read()

# Let's count occurrences of `args.get(X)`
print(f"args.get count: {code.count('args.get(')}")

matches = re.finditer(r'match args\.get\(([^)]+)\) \{', code)
for m in matches:
    start_pos = m.start()
    line_num = code[:start_pos].count('\n') + 1
    # Grab the next 5 lines
    lines = code[start_pos:start_pos+300].split('\n')[:8]
    print(f"Line {line_num}:")
    for l in lines:
        print("  " + l)
    print()

