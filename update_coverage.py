with open("crates/duke-interpreter/Cargo.toml", "r") as f:
    code = f.read()

code += """
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(tarpaulin_include)'] }
"""

with open("crates/duke-interpreter/Cargo.toml", "w") as f:
    f.write(code)
