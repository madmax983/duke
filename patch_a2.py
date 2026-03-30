import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    code = f.read()

# For extract_string_arg_value
code = re.sub(
    r'let str_ref = match args\.get\(([^)]+)\) \{[^{}]*Some\(Slot::Reference\(Some\(r\)\)\) => \*r,[^{}]*Some\(Slot::Reference\(None\)\) => return Err\(VmError::NullPointerException\),[^{}]*_ => \{[^{}]*return Err\(VmError::TypeMismatch \{[^{}]*expected: "Reference",[^{}]*got: "other",[^{}]*\}[^{}]*\);[^{}]*\}[^{}]*\};',
    r'let str_ref = extract_ref_arg(args, \1)?;',
    code
)

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.write(code)
