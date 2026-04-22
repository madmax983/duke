import re

with open("crates/duke-bytecode/src/instruction.rs", "r") as f:
    data = f.read()

data = data.replace("*current += 1;", "*current = current.wrapping_add(1);")

with open("crates/duke-bytecode/src/instruction.rs", "w") as f:
    f.write(data)
