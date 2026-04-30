with open("crates/duke-bytecode/src/instruction.rs", "r") as f:
    content = f.read()

content = content.replace("#[must_use]\n    #[must_use]", "#[must_use]")

with open("crates/duke-bytecode/src/instruction.rs", "w") as f:
    f.write(content)
