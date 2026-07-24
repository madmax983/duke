import os
import re

def find_missing_docs(directory):
    for root, _, files in os.walk(directory):
        for file in files:
            if file.endswith(".rs"):
                filepath = os.path.join(root, file)
                with open(filepath, 'r') as f:
                    content = f.read()

                    # Split into lines
                    lines = content.split('\n')
                    for i, line in enumerate(lines):
                        if line.strip().startswith('pub fn') or line.strip().startswith('pub const fn') or line.strip().startswith('pub async fn'):
                            # Check if the previous lines contain documentation or macros
                            # A simple check: Look at the previous line
                            j = i - 1
                            has_doc = False
                            while j >= 0:
                                prev_line = lines[j].strip()
                                if prev_line.startswith('///'):
                                    has_doc = True
                                    break
                                elif prev_line.startswith('#[') or prev_line == '':
                                    j -= 1
                                else:
                                    break

                            if not has_doc:
                                print(f"{filepath}:{i+1} - {line.strip()}")

find_missing_docs('crates')
