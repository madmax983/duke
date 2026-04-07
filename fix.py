import sys

with open('duke/src/main.rs', 'r') as f:
    content = f.read()

content = content.replace('#[allow(clippy::too_many_lines)]\n#[cfg(not(tarpaulin_include))]', '#[allow(clippy::too_many_lines)]\n#[allow(unexpected_cfgs)]\n#[cfg(not(tarpaulin_include))]')

with open('duke/src/main.rs', 'w') as f:
    f.write(content)
