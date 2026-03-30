import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    code = f.read()

# Replace match args.get(X) blocks for string ref extraction where there's no helper
code = re.sub(
    r'match args\.get\(([^)]+)\) \{\n\s*Some\(Slot::Reference\(Some\(([^)]+)\)\)\) => \*\2,\n\s*Some\(Slot::Reference\(None\)\) => return Err\(VmError::NullPointerException\),\n\s*_ => \{\n\s*return Err\(VmError::TypeMismatch \{\n\s*expected: "Reference",\n\s*got: "other",\n\s*\}\);\n\s*\}\n\s*\}',
    r'extract_ref_arg(args, \1)?',
    code
)

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.write(code)
