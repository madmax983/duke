import re

def get_functions_from_file(filename):
    functions = []
    in_function = False
    current_func = []
    brace_count = 0
    start_line = 0
    func_name = ""
    attr_lines = []

    with open(filename, 'r') as f:
        for i, line in enumerate(f):
            if not in_function:
                # keep track of docstrings/attributes before function
                if re.match(r'^\s*///|^\s*#\[', line):
                    attr_lines.append(line)
                    if start_line == 0:
                        start_line = i + 1
                    continue

                match = re.search(r'(?:pub\s+)?(?:unsafe\s+)?fn\s+([a-zA-Z_0-9]+)\s*\(', line)
                if match:
                    in_function = True
                    if start_line == 0:
                        start_line = i + 1
                    func_name = match.group(1)
                    current_func = attr_lines + [line]
                    attr_lines = []
                    brace_count += line.count('{') - line.count('}')

                    if brace_count == 0 and '{' in line and '}' in line:
                        functions.append((start_line, i + 1, func_name, "".join(current_func)))
                        in_function = False
                        current_func = []
                        func_name = ""
                        start_line = 0
                else:
                    attr_lines = []
                    start_line = 0
            else:
                current_func.append(line)
                brace_count += line.count('{') - line.count('}')
                if brace_count == 0:
                    functions.append((start_line, i + 1, func_name, "".join(current_func)))
                    in_function = False
                    current_func = []
                    func_name = ""
                    start_line = 0
    return functions

fns = get_functions_from_file("crates/duke-interpreter/src/lib.rs")
res = []
for start, end, name, code in fns:
    if "native_" in name and not "test" in name and not "telemetry" in name and not "registry" in name and not "native_collections_sort" in name:
        res.append((start, end, name, code))
    elif name in ["heap_object_to_string", "ordering_to_int", "format_java_double", "format_java_float", "stringify_slot", "format_arg"]:
        res.append((start, end, name, code))

to_remove = set()
for start, end, name, code in res:
    if not code.strip().endswith('}'):
        continue
    safe = True
    for i in range(start, end + 1):
        if i < 1604: # Do not remove anything inside bootstrap_stdlib
            safe = False
            break
    if safe:
        for i in range(start, end + 1):
            to_remove.add(i)

print(f"Total functions to remove: {len(res)}, Total lines to remove: {len(to_remove)}")
import os
os.makedirs("crates/duke-interpreter/src/native", exist_ok=True)

with open("crates/duke-interpreter/src/native/mod.rs", "w") as f:
    f.write("use std::io::Write;\n")
    f.write("use duke_runtime::{Slot, VmError, VmResult};\n")
    f.write("use duke_gc::Heap;\n")
    f.write("use duke_gc::HeapObject;\n")
    f.write("\n")
    for start, end, name, code in res:
        safe = True
        for i in range(start, end + 1):
            if i < 1604:
                safe = False
                break
        if not safe: continue
        if not code.strip().endswith('}'):
            continue

        code = re.sub(r'^fn ', 'pub(crate) fn ', code)
        code = re.sub(r'(\n)fn ', r'\1pub(crate) fn ', code)
        f.write(code + "\n")

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    lines = f.readlines()

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    for i, line in enumerate(lines):
        if i == 10: # Just after some imports
            f.write("\npub(crate) mod native;\n")
            f.write("use native::*;\n\n")

        if (i + 1) in to_remove:
            continue
        f.write(line)
