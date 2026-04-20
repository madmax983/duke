import os
import re

def check_file(filepath):
    try:
        with open(filepath, 'r') as f:
            lines = f.readlines()
    except Exception:
        return

    for i, line in enumerate(lines):
        if re.match(r'^\s*pub fn', line):
            # Check the lines above it for /// or //!
            has_doc = False
            for j in range(max(0, i-15), i):
                if '///' in lines[j] or '//! ' in lines[j] or '/**' in lines[j]:
                    has_doc = True
                    break

            if not has_doc:
                print(f"No doc: {filepath}:{i+1} {line.strip()}")

for root, dirs, files in os.walk('crates'):
    for file in files:
        if file.endswith('.rs') and 'tests' not in root and 'fuzz' not in file:
            check_file(os.path.join(root, file))
