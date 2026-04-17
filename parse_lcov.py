import re
from collections import defaultdict

with open('lcov.info', 'r') as f:
    lines = f.readlines()

current_file = None
uncovered = defaultdict(list)

for line in lines:
    line = line.strip()
    if line.startswith('SF:'):
        current_file = line[3:]
    elif line.startswith('DA:'):
        parts = line[3:].split(',')
        line_num = int(parts[0])
        hits = int(parts[1])
        if hits == 0:
            uncovered[current_file].append(line_num)

for file, lines in uncovered.items():
    if lines and 'telemetry' in file:
        print(f"{file}: {len(lines)} uncovered lines")
        print(f"  first few: {lines[:10]}")
