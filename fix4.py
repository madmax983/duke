import sys

with open('duke/src/main.rs', 'r') as f:
    content = f.read()

content = content.replace('#[allow(unexpected_cfgs)]\n#[cfg(not(tarpaulin_include))]', '#[allow(unexpected_cfgs)]\n#[cfg(not(tarpaulin_include))]')

with open('duke/src/main.rs', 'w') as f:
    f.write(content)

with open('duke/Cargo.toml', 'r') as f:
    content = f.read()

content += '\n[lints.rust]\nunexpected_cfgs = { level = "warn", check-cfg = [\'cfg(tarpaulin_include)\'] }\n'
with open('duke/Cargo.toml', 'w') as f:
    f.write(content)
