import sys

with open('duke/src/main.rs', 'r') as f:
    content = f.read()

content = content.replace('#[allow(unexpected_cfgs)]\n#[cfg(not(tarpaulin_include))]', '#[cfg_attr(not(tarpaulin_include), allow(unexpected_cfgs))]\n#[cfg(not(tarpaulin_include))]')

with open('duke/src/main.rs', 'w') as f:
    f.write(content)
