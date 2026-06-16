import re

with open('crates/duke-interpreter/src/native.rs', 'r') as f:
    lines = f.readlines()

for i in range(len(lines)):
    line = lines[i]
    if "let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();" in line:
        pass # we can change this to two lines
