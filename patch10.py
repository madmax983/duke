import re
import sys

with open("crates/duke-interpreter/src/natives.rs", "r") as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if line.startswith("fn execute_string_concat_recipe"):
        lines[i] = "pub " + line
    elif line.startswith("fn stringify_slot"):
        lines[i] = "pub " + line
    elif line.startswith("const THREAD_TARGET_SLOT"):
        lines[i] = "pub " + line
    elif line.startswith("const THREAD_ID_SLOT"):
        lines[i] = "pub " + line
    elif line.startswith("const COMPARE_TO_METHOD"):
        lines[i] = "pub " + line
    elif line.startswith("const COMPARE_TO_OBJECT_DESC"):
        lines[i] = "pub " + line
    elif line.startswith("const SORT_COMPARATOR_DESC"):
        lines[i] = "pub " + line
    elif line.startswith("const fn ordering_to_int"):
        lines[i] = "pub " + line

with open("crates/duke-interpreter/src/natives.rs", "w") as f:
    f.writelines(lines)
