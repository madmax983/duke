import re
import os
from collections import defaultdict

# Start over carefully to avoid test issues
with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    lines = f.readlines()

test_start = 12693
main_lines = lines[:test_start]
test_lines = lines[test_start:]

# Find native blocks
natives = []
in_native = False
current_native = []
brace_count = 0
start_line = 0

for i, line in enumerate(main_lines):
    if not in_native:
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

            in_native = True
            current_native = main_lines[start_idx:i+1]
            brace_count = line.count('{') - line.count('}')
            start_line = start_idx
    else:
        current_native.append(line)
        brace_count += line.count('{') - line.count('}')
        if brace_count <= 0:
            func_name = None
            for cl in current_native:
                m = re.search(r'fn\s+(native_[a-zA-Z0-9_]+)', cl)
                if m:
                    func_name = m.group(1)
                    break

            if func_name:
                natives.append({
                    'name': func_name,
                    'start': start_line,
                    'end': i,
                    'code': "".join(current_native).strip()
                })
            in_native = False

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

# Make items public crate so they can be imported
pub_items = [
    'class_internal_name_from_ref', 'internal_name_to_binary_name', 'CallbackOps', 'binary_name_to_internal_name',
    'allocate_class_object', 'allocate_reflection_member_object', 'allocate_reference_array', 'reflection_member_name_slot',
    'reflection_array_elements', 'reflected_method_handle', 'build_reflection_invoke_args', 'find_hashmap_entry_index',
    'find_hashset_entry_index', 'file_path_from_this', 'path_from_string_slot', 'file_stream_id_from_this', 'ordering_to_int',
    'format_arg', 'slot_to_char', 'box_reflection_return_value', 'descriptor_return_type'
]

# We need to apply regex replacements to lib.rs content directly
lib_content = "".join(main_lines)

for item in pub_items:
    if item == 'CallbackOps':
        lib_content = re.sub(r'\btrait ' + item + r'\b', f'pub(crate) trait {item}', lib_content)
    else:
        lib_content = re.sub(r'\bfn ' + item + r'\b', f'pub(crate) fn {item}', lib_content)

for item in ['THREAD_TARGET_SLOT', 'THREAD_ID_SLOT', 'FILE_DESCRIPTOR_SLOT', 'SORT_COMPARATOR_DESC']:
    lib_content = re.sub(r'\bconst ' + item + r'\b', f'pub(crate) const {item}', lib_content)

lib_content = re.sub(r'\benum FileDescriptor\b', f'pub(crate) enum FileDescriptor', lib_content)

# We also need to add pub mod natives;
lib_content = lib_content.replace('pub mod registry;\n', 'pub mod registry;\npub mod natives;\n')

# Now remove the native definitions from main_lines safely
to_remove = set()
for n in natives:
    for i in range(n['start'], n['end'] + 1):
        to_remove.add(i)

final_main = []
# Need to apply the removals on the ORIGINAL main_lines so we don't mess up line numbers,
# but we just did replacements on the whole text.
# Better approach: Just do removals on lines, then do regexes.
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
lib_content = lib_content.replace('pub mod registry;\n', 'pub mod registry;\npub mod natives;\n')

# Instead of updating ALL tests, we just import the modules in the tests block.
# Tests often use `wrap_simple_native_for_tests!`. We must NOT modify the macro!
# We just need to make sure the native functions are accessible without `crate::natives::...` in `lib.rs`.
# By putting `use crate::natives::*` at the top of lib.rs?
# No, we can just `pub use crate::natives::*` or explicitly map them where they are needed in the macro invocation.
# Wait, if we leave `native_` function calls as they are inside `lib.rs` (e.g. bootstrap), we need them in scope.
# We can just add `use crate::natives::*::*;`? Rust doesn't allow `*::*`.
# We will just write a block of uses.

use_block = "\n"
for cat in categorized.keys():
    use_block += f"use crate::natives::{cat}::*;\n"

lib_content = lib_content.replace('pub mod registry;\npub mod natives;\n', f'pub mod registry;\npub mod natives;\n{use_block}\n')

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.write(lib_content)
    f.write("".join(test_lines))

print("Rewritten securely.")
