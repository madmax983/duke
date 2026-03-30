import re
import os
from collections import defaultdict

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    lines = f.readlines()

test_start = 12693
main_lines = lines[:test_start]
test_lines = lines[test_start:]

natives = []
i = 0
while i < len(main_lines):
    line = main_lines[i]
    if line.startswith('fn native_') or line.startswith('pub fn native_') or line.startswith('pub(crate) fn native_') or re.match(r'^(#\[allow\(.*?\)\]\s*)?fn native_', line):
        start_idx = i
        while start_idx > 0:
            prev = main_lines[start_idx-1].strip()
            if prev.startswith('///') or prev.startswith('#[') or prev.startswith('//'):
                start_idx -= 1
            elif not prev:
                start_idx -= 1
            else:
                break

        # Find the function body start `{`
        brace_count = 0
        in_body = False
        j = start_idx
        while j < len(main_lines):
            curr_line = main_lines[j]
            if not in_body and '{' in curr_line:
                in_body = True

            if in_body:
                brace_count += curr_line.count('{') - curr_line.count('}')
                if brace_count <= 0:
                    break
            j += 1

        end_idx = j

        func_name = None
        for k in range(start_idx, end_idx + 1):
            m = re.search(r'fn\s+(native_[a-zA-Z0-9_]+)', main_lines[k])
            if m:
                func_name = m.group(1)
                break

        if func_name:
            natives.append({
                'name': func_name,
                'start': start_idx,
                'end': end_idx,
                'code': "".join(main_lines[start_idx:end_idx+1]).strip()
            })

        i = end_idx + 1
    else:
        i += 1

categories = {
    'io': ['native_println_', 'native_print_', 'native_file_', 'native_socket_', 'native_server_socket_'],
    'zip': ['native_zip_'],
    'string': ['native_string_', 'native_sb_', 'native_char_'],
    'object': ['native_object_', 'native_class_', 'native_reflect_', 'native_enum_', 'native_throwable_'],
    'math': ['native_math_', 'native_integer_', 'native_long_', 'native_float_', 'native_double_', 'native_boolean_'],
    'thread': ['native_thread_'],
    'collection': ['native_arraylist_', 'native_hashmap_', 'native_arrays_', 'native_collections_', 'native_hashset_'],
    'util': ['native_system_', 'native_time_']
}

categorized = defaultdict(list)
for n in natives:
    placed = False
    for cat, prefixes in categories.items():
        if any(n['name'].startswith(p) for p in prefixes):
            categorized[cat].append(n)
            placed = True
            break
    if not placed:
        categorized['misc'].append(n)

os.makedirs("crates/duke-interpreter/src/natives", exist_ok=True)

imports = """use std::io::Write;
use duke_runtime::{Slot, VmError, VmResult};
use crate::{NativeControl, heap_object_to_string};
use crate::{class_internal_name_from_ref, internal_name_to_binary_name, CallbackOps, binary_name_to_internal_name, allocate_class_object, allocate_reflection_member_object, allocate_reference_array, reflection_member_name_slot, reflection_array_elements, reflected_method_handle, build_reflection_invoke_args, find_hashmap_entry_index, find_hashset_entry_index, file_path_from_this, path_from_string_slot, file_stream_id_from_this, ordering_to_int, format_arg, slot_to_char, box_reflection_return_value, descriptor_return_type};
use crate::{THREAD_ID_SLOT, THREAD_TARGET_SLOT, FILE_DESCRIPTOR_SLOT, SORT_COMPARATOR_DESC, NativeThreadAction, FileDescriptor};
use duke_gc::Heap;
"""

for cat, fns in categorized.items():
    if not fns: continue
    with open(f"crates/duke-interpreter/src/natives/{cat}.rs", "w") as f:
        f.write(imports + "\n")
        for n in fns:
            code = re.sub(r'\bfn native_', 'pub(crate) fn native_', n['code'])
            f.write(code + "\n\n")

with open("crates/duke-interpreter/src/natives/mod.rs", "w") as f:
    for cat in sorted(categorized.keys()):
        if categorized[cat]:
            f.write(f"pub mod {cat};\n")

# Need to also extract bootstrap_stdlib into natives/mod.rs
# But we can just leave it in lib.rs for now to reduce complexity, since the user only asked for `fn native_` to be extracted.

pub_items = [
    'class_internal_name_from_ref', 'internal_name_to_binary_name', 'CallbackOps', 'binary_name_to_internal_name',
    'allocate_class_object', 'allocate_reflection_member_object', 'allocate_reference_array', 'reflection_member_name_slot',
    'reflection_array_elements', 'reflected_method_handle', 'build_reflection_invoke_args', 'find_hashmap_entry_index',
    'find_hashset_entry_index', 'file_path_from_this', 'path_from_string_slot', 'file_stream_id_from_this', 'ordering_to_int',
    'format_arg', 'slot_to_char', 'box_reflection_return_value', 'descriptor_return_type', 'heap_object_to_string'
]

# Write lib.rs securely
to_remove = set()
for n in natives:
    for i in range(n['start'], n['end'] + 1):
        to_remove.add(i)

main_lines_after_removal = [line for i, line in enumerate(main_lines) if i not in to_remove]
lib_content = "".join(main_lines_after_removal)

for item in pub_items:
    if item == 'CallbackOps':
        lib_content = re.sub(r'\btrait ' + item + r'\b', f'pub(crate) trait {item}', lib_content)
    else:
        lib_content = re.sub(r'\bfn ' + item + r'\b', f'pub(crate) fn {item}', lib_content)

for item in ['THREAD_TARGET_SLOT', 'THREAD_ID_SLOT', 'FILE_DESCRIPTOR_SLOT', 'SORT_COMPARATOR_DESC']:
    lib_content = re.sub(r'\bconst ' + item + r'\b', f'pub(crate) const {item}', lib_content)

lib_content = re.sub(r'\benum FileDescriptor\b', f'pub(crate) enum FileDescriptor', lib_content)

use_block = "\n"
for cat in categorized.keys():
    if categorized[cat]:
        use_block += f"use crate::natives::{cat}::*;\n"

lib_content = lib_content.replace('pub mod registry;\n', f'pub mod registry;\npub mod natives;\n{use_block}\n')

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.write(lib_content)
    f.write("".join(test_lines))

print(f"Extracted {len(natives)} functions reliably.")
