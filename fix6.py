with open('Cargo.toml', 'r') as f:
    content = f.read()

content = content.replace('[workspace.lints.clippy]\npedantic = { level = "warn", priority = -1 }\nnursery = { level = "warn", priority = -1 }\n\n[profile.profiling]\ninherits = "release"\ndebug = 1\n\n[workspace.lints.rust]\nunexpected_cfgs = { level = "warn", check-cfg = [\'cfg(tarpaulin_include)\'] }\n', '[workspace.lints.clippy]\npedantic = { level = "warn", priority = -1 }\nnursery = { level = "warn", priority = -1 }\n\n[workspace.lints.rust]\nunexpected_cfgs = { level = "warn", check-cfg = [\'cfg(tarpaulin_include)\'] }\n\n[profile.profiling]\ninherits = "release"\ndebug = 1\n')

with open('Cargo.toml', 'w') as f:
    f.write(content)
