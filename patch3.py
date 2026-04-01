import re
import sys

with open("crates/duke-interpreter/src/natives.rs", "r") as f:
    lines = f.readlines()

with open("crates/duke-interpreter/src/natives.rs", "w") as f:
    # Remove the dangling attribute at the end
    f.writelines(lines[:-22])
