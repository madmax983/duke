import sys

with open('Cargo.toml', 'r') as f:
    content = f.read()

content += '\n[workspace.lints.rust]\nunexpected_cfgs = { level = "warn", check-cfg = [\'cfg(tarpaulin_include)\'] }\n'
with open('Cargo.toml', 'w') as f:
    f.write(content)

with open('duke/src/main.rs', 'r') as f:
    content = f.read()
content = content.replace('#[allow(unexpected_cfgs)]\n#[cfg(not(tarpaulin_include))]', '#[cfg(not(tarpaulin_include))]')
with open('duke/src/main.rs', 'w') as f:
    f.write(content)
