with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if line.startswith("mod tests {"):
        start_idx = i
        break

balance = 0
found = False
end_idx = 0
for i in range(start_idx, len(lines)):
    line = lines[i]
    for char in line:
        if char == '{':
            balance += 1
            found = True
        elif char == '}':
            balance -= 1
            if found and balance == 0:
                end_idx = i
                break
    if found and balance == 0:
        break

tests_lines = lines[start_idx+1:end_idx] # skip mod tests { and }

with open("crates/duke-interpreter/src/tests.rs", "w") as f:
    f.writelines(tests_lines)

# Now put it back but use include!
new_lib = lines[:start_idx+1] + ["    include!(\"tests.rs\");\n"] + lines[end_idx:]

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.writelines(new_lib)
