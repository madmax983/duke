import re
content = open('crates/duke-interpreter/src/lib.rs').read()
content = content.replace('triggered_by: &str,', '_triggered_by: &str,')
content = content.replace('triggered_by,', '_triggered_by,')
content = content.replace('__triggered_by,', '_triggered_by,')
content = content.replace('__triggered_by', '_triggered_by')
open('crates/duke-interpreter/src/lib.rs', 'w').write(content)
