import re
import sys

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    lines = f.readlines()

bootstrap_start = -1
bootstrap_end = -1
for i, line in enumerate(lines):
    if "pub fn bootstrap_stdlib" in line:
        bootstrap_start = i
    if bootstrap_start != -1 and line.startswith("pub fn execute("):
        bootstrap_end = i
        break

if bootstrap_start == -1 or bootstrap_end == -1:
    print("Could not find start or end")
    sys.exit(1)

print(f"Bootstrap is between {bootstrap_start} and {bootstrap_end}")
