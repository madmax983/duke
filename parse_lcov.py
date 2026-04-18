import re

with open("lcov.info", "r") as f:
    lines = f.readlines()

current_file = None
for line in lines:
    line = line.strip()
    if line.startswith("SF:"):
        current_file = line[3:]
    elif line.startswith("DA:") and line.endswith(",0"):
        if "duke-telemetry/src/lib.rs" in current_file or "duke-interpreter/src/native.rs" in current_file:
            print(f"{current_file}:{line[3:].split(',')[0]}")
