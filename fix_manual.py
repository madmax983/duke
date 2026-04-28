import re

# Update opcodes.rs
with open("crates/duke-bytecode/src/opcodes.rs", "r") as f:
    content = f.read()

content = content.replace("#![allow(missing_docs)]\n", "")

with open("crates/duke-bytecode/src/opcodes.rs", "w") as f:
    f.write(content)

# Update instruction.rs
with open("crates/duke-bytecode/src/instruction.rs", "r") as f:
    content = f.read()

content = content.replace("#![allow(missing_docs)]\n", "")

content = content.replace("pub enum Instruction {", "#[allow(missing_docs)]\npub enum Instruction {")
content = content.replace("pub enum ArrayType {", "#[allow(missing_docs)]\npub enum ArrayType {")

with open("crates/duke-bytecode/src/instruction.rs", "w") as f:
    f.write(content)
