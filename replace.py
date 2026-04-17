import re

with open('crates/duke-interpreter/src/native.rs', 'r') as f:
    content = f.read()

content = re.sub(
    r'fn extract_int_arg\(args: &\[Slot\], idx: usize\) -> VmResult<i32> \{\n\s*let slot = args\.get\(idx\)\.copied\(\)\.unwrap_or\(Slot::Reference\(None\)\);\n\s*match slot\.as_int\(\) \{\n\s*Ok\(v\) => Ok\(v\),\n\s*Err\(\_\) => Err\(VmError::TypeMismatch \{ expected: "Int", got: "other" \}\),\n\s*\}\n\}',
    'fn extract_int_arg(args: &[Slot], idx: usize) -> VmResult<i32> {\n    args.get(idx).copied().unwrap_or(Slot::Reference(None)).as_int().map_err(|_| VmError::TypeMismatch { expected: "Int", got: "other" })\n}',
    content
)

content = re.sub(
    r'fn extract_long_arg\(args: &\[Slot\], idx: usize\) -> VmResult<i64> \{\n\s*let slot = args\.get\(idx\)\.copied\(\)\.unwrap_or\(Slot::Reference\(None\)\);\n\s*match slot\.as_long\(\) \{\n\s*Ok\(v\) => Ok\(v\),\n\s*Err\(\_\) => Err\(VmError::TypeMismatch \{ expected: "Long", got: "other" \}\),\n\s*\}\n\}',
    'fn extract_long_arg(args: &[Slot], idx: usize) -> VmResult<i64> {\n    args.get(idx).copied().unwrap_or(Slot::Reference(None)).as_long().map_err(|_| VmError::TypeMismatch { expected: "Long", got: "other" })\n}',
    content
)

content = re.sub(
    r'fn extract_float_arg\(args: &\[Slot\], idx: usize\) -> VmResult<f32> \{\n\s*let slot = args\.get\(idx\)\.copied\(\)\.unwrap_or\(Slot::Reference\(None\)\);\n\s*match slot\.as_float\(\) \{\n\s*Ok\(v\) => Ok\(v\),\n\s*Err\(\_\) => Err\(VmError::TypeMismatch \{ expected: "Float", got: "other" \}\),\n\s*\}\n\}',
    'fn extract_float_arg(args: &[Slot], idx: usize) -> VmResult<f32> {\n    args.get(idx).copied().unwrap_or(Slot::Reference(None)).as_float().map_err(|_| VmError::TypeMismatch { expected: "Float", got: "other" })\n}',
    content
)

content = re.sub(
    r'fn extract_double_arg\(args: &\[Slot\], idx: usize\) -> VmResult<f64> \{\n\s*let slot = args\.get\(idx\)\.copied\(\)\.unwrap_or\(Slot::Reference\(None\)\);\n\s*match slot\.as_double\(\) \{\n\s*Ok\(v\) => Ok\(v\),\n\s*Err\(\_\) => Err\(VmError::TypeMismatch \{ expected: "Double", got: "other" \}\),\n\s*\}\n\}',
    'fn extract_double_arg(args: &[Slot], idx: usize) -> VmResult<f64> {\n    args.get(idx).copied().unwrap_or(Slot::Reference(None)).as_double().map_err(|_| VmError::TypeMismatch { expected: "Double", got: "other" })\n}',
    content
)

with open('crates/duke-interpreter/src/native.rs', 'w') as f:
    f.write(content)
