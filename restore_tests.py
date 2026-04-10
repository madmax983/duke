# The tests that were removed were the host specific tests in `lib.rs`.
# I will fetch them from the original `lib.rs` and put them back in `host.rs` with `HostManager::new()`.

import subprocess
import re

# Fetch the original tests from trunk
orig = subprocess.run("git show origin/trunk:crates/duke-gc/src/lib.rs", shell=True, capture_output=True, text=True).stdout

tests_to_move = [
    "fn open_host_input_file_not_found()",
    "fn spawn_host_process_empty_command()",
    "fn spawn_host_process_invalid_command()",
    "fn read_write_invalid_host_file_handle()",
    "fn should_handle_open_read_write_close_cycle()",
    "fn test_host_file_operations()",
    "fn process_wait_and_destroy_cycle()",
    "fn socket_operations()",
]

test_blocks = []

for test in tests_to_move:
    # Use exact match brace counting parser instead of regex to avoid mismatch
    start_idx = orig.find("#[test]\nfn " + test[3:])
    if start_idx == -1:
        # Might be `#[test]\nfn ...` with indents? No, they are at the root level.
        start_idx = orig.find(test)
        if start_idx != -1:
            start_idx = orig.rfind("#[test]", 0, start_idx)

    if start_idx != -1:
        # Find closing brace
        open_braces = 0
        started = False
        end_idx = start_idx
        for i in range(start_idx, len(orig)):
            if orig[i] == '{':
                open_braces += 1
                started = True
            elif orig[i] == '}':
                open_braces -= 1
            if started and open_braces == 0:
                end_idx = i + 1
                break

        block = orig[start_idx:end_idx]
        test_blocks.append(block)

adapted_blocks = []
for block in test_blocks:
    b = block.replace("let mut heap = Heap::new();", "let mut host = HostManager::new();\n    let mut _heap = Heap::new();")
    b = b.replace("let mut gc = Heap::new();", "let mut gc = HostManager::new();")
    b = b.replace("heap.host.", "host.")
    b = b.replace("gc.host.", "gc.")
    b = b.replace("heap.", "host.")
    adapted_blocks.append(b)

test_module = "\n#[cfg(test)]\nmod host_tests {\n    use super::*;\n    use crate::Heap;\n\n"
for b in adapted_blocks:
    indented = "\n".join("    " + line for line in b.splitlines())
    test_module += indented + "\n\n"
test_module += "}\n"

with open("crates/duke-gc/src/host.rs", "a") as f:
    f.write(test_module)

print("Done restoring tests cleanly")
