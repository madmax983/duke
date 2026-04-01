import re
import sys

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    lines = f.readlines()

bootstrap_start = -1
bootstrap_end = -1
for i, line in enumerate(lines):
    if "pub fn bootstrap_stdlib" in line:
        bootstrap_start = i
    if "pub fn execute(" in line:
        bootstrap_end = i
        break

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.writelines(lines[:bootstrap_start])
    f.writelines(lines[bootstrap_end:])
