import re
content = open('crates/duke-interpreter/src/lib.rs').read()
content = content.replace('_triggered_by: &str,', '#[allow(unused_variables)] triggered_by: &str,')
content = content.replace('            _triggered_by,', '            triggered_by,')
open('crates/duke-interpreter/src/lib.rs', 'w').write(content)
