import sys
import re

def get_lines(filename, start_line, end_line):
    with open(filename, 'r') as f:
        lines = f.readlines()
        for i in range(start_line - 1, min(end_line, len(lines))):
            print(f"{i + 1:4d}: {lines[i]}", end='')

get_lines('crates/duke-telemetry/src/lib.rs', 100, 150)
