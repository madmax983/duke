import re

with open('crates/duke-interpreter/src/native.rs', 'r') as f:
    content = f.read()

# Fix the VmResult missing in native.rs test (line 25120)
content = re.sub(r'let handle = thread::spawn\(move \|\| -> VmResult<\(\)> \{', r'let handle = thread::spawn(move || -> crate::Result<()> {', content)

with open('crates/duke-interpreter/src/native.rs', 'w') as f:
    f.write(content)
