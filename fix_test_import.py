import re
with open('crates/duke-interpreter/src/lib.rs', 'r') as f:
    content = f.read()
content = content.replace("use crate::*;", "")
with open('crates/duke-interpreter/src/lib.rs', 'w') as f:
    f.write(content)
