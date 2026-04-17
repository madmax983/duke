import re

with open('crates/duke-interpreter/src/native.rs', 'r') as f:
    content = f.read()

content = re.sub(
    r'fn extract_int_arg\(args: &\[Slot\], idx: usize\) -> VmResult<i32> \{\n\s*let slot = args\.get\(idx\)\.copied\(\)\.unwrap_or\(Slot::Reference\(None\)\);\n\s*match slot\.as_int\(\) \{\n\s*Ok\(v\) => Ok\(v\),\n\s*Err\(\_\) => Err\(VmError::TypeMismatch \{ expected: "Int", got: "other" \}\),\n\s*\}\n\}',
    'fn extract_int_arg(args: &[Slot], idx: usize) -> VmResult<i32> {\n    match args.get(idx).copied().unwrap_or(Slot::Reference(None)) {\n        Slot::Int(v) => Ok(v),\n        _ => Err(VmError::TypeMismatch { expected: "Int", got: "other" }),\n    }\n}',
    content
)

content = re.sub(
    r'fn extract_long_arg\(args: &\[Slot\], idx: usize\) -> VmResult<i64> \{\n\s*let slot = args\.get\(idx\)\.copied\(\)\.unwrap_or\(Slot::Reference\(None\)\);\n\s*match slot\.as_long\(\) \{\n\s*Ok\(v\) => Ok\(v\),\n\s*Err\(\_\) => Err\(VmError::TypeMismatch \{ expected: "Long", got: "other" \}\),\n\s*\}\n\}',
    'fn extract_long_arg(args: &[Slot], idx: usize) -> VmResult<i64> {\n    match args.get(idx).copied().unwrap_or(Slot::Reference(None)) {\n        Slot::Long(v) => Ok(v),\n        _ => Err(VmError::TypeMismatch { expected: "Long", got: "other" }),\n    }\n}',
    content
)

content = re.sub(
    r'fn extract_float_arg\(args: &\[Slot\], idx: usize\) -> VmResult<f32> \{\n\s*let slot = args\.get\(idx\)\.copied\(\)\.unwrap_or\(Slot::Reference\(None\)\);\n\s*match slot\.as_float\(\) \{\n\s*Ok\(v\) => Ok\(v\),\n\s*Err\(\_\) => Err\(VmError::TypeMismatch \{ expected: "Float", got: "other" \}\),\n\s*\}\n\}',
    'fn extract_float_arg(args: &[Slot], idx: usize) -> VmResult<f32> {\n    match args.get(idx).copied().unwrap_or(Slot::Reference(None)) {\n        Slot::Float(v) => Ok(v),\n        _ => Err(VmError::TypeMismatch { expected: "Float", got: "other" }),\n    }\n}',
    content
)

content = re.sub(
    r'fn extract_double_arg\(args: &\[Slot\], idx: usize\) -> VmResult<f64> \{\n\s*let slot = args\.get\(idx\)\.copied\(\)\.unwrap_or\(Slot::Reference\(None\)\);\n\s*match slot\.as_double\(\) \{\n\s*Ok\(v\) => Ok\(v\),\n\s*Err\(\_\) => Err\(VmError::TypeMismatch \{ expected: "Double", got: "other" \}\),\n\s*\}\n\}',
    'fn extract_double_arg(args: &[Slot], idx: usize) -> VmResult<f64> {\n    match args.get(idx).copied().unwrap_or(Slot::Reference(None)) {\n        Slot::Double(v) => Ok(v),\n        _ => Err(VmError::TypeMismatch { expected: "Double", got: "other" }),\n    }\n}',
    content
)


with open('crates/duke-interpreter/src/native.rs', 'w') as f:
    f.write(content)
