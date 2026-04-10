import re

with open('crates/duke-gc/src/lib.rs', 'r') as f:
    content = f.read()

# Find where HostProcessHandle starts.
start_idx = content.find("#[derive(Debug)]\n/// A handle for a native host process managed by the VM.")

if start_idx == -1:
    print("Could not find HostProcessHandle")
    exit(1)

# Find where HostFileHandle ends.
end_idx = content.find("pub enum HostFileHandle {")
end_idx = content.find("}\n\n/// The generational object heap.", end_idx) + 2

types_block = content[start_idx:end_idx]

# Now let's find the start of the host methods in impl Heap.
method_start = content.find("    /// Opens an input file on the host OS.")

# The last method is promote_to_old, so the host methods end right before it.
method_end = content.find("    /// Promote a young-gen object to old gen.")

methods_block = content[method_start:method_end]

# Imports needed
imports = """use std::collections::{HashMap};
use std::io::{Read, Write};

use duke_runtime::{VmError, VmResult};
use duke_loader::ZipReader;
"""

# Let's create HostManager struct
host_manager_struct = """
/// Manages host OS file and process handles for the VM.
#[derive(Debug)]
pub struct HostManager {
    /// Host OS file handles keyed by small integer ids stored in Java objects.
    pub host_files: HashMap<i32, HostFileHandle>,
    pub next_host_file_id: i32,
}

impl Default for HostManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HostManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            host_files: HashMap::new(),
            next_host_file_id: 1,
        }
    }
"""

host_rs = imports + "\n" + types_block + host_manager_struct + "\n" + methods_block + "}\n"

with open('crates/duke-gc/src/host.rs', 'w') as f:
    f.write(host_rs)

# Now modify lib.rs
new_lib = content[:start_idx] + content[end_idx:]
# We need to insert `pub mod host;` and `pub use host::*;`
insert_idx = new_lib.find("mod mermaid;")
new_lib = new_lib[:insert_idx] + "pub mod host;\npub use host::*;\n" + new_lib[insert_idx:]

# And remove the methods
method_start = new_lib.find("    /// Opens an input file on the host OS.")
method_end = new_lib.find("    /// Promote a young-gen object to old gen.")
new_lib = new_lib[:method_start] + new_lib[method_end:]

# Replace host_files and next_host_file_id in Heap with pub host: HostManager
new_lib = new_lib.replace("    host_files: HashMap<i32, HostFileHandle>,\n    next_host_file_id: i32,", "    pub host: HostManager,")
new_lib = new_lib.replace("            host_files: HashMap::new(),\n            next_host_file_id: 1,", "            host: HostManager::new(),")

with open('crates/duke-gc/src/lib.rs', 'w') as f:
    f.write(new_lib)

# Now update duke-interpreter/src/lib.rs
with open('crates/duke-interpreter/src/lib.rs', 'r') as f:
    interpreter_content = f.read()

# Replace all heap.host_method(...) with heap.host.host_method(...)
methods_to_replace = [
    "open_host_input_file",
    "open_host_output_file",
    "read_host_file_byte",
    "write_host_file_byte",
    "close_host_file",
    "spawn_host_process",
    "wait_host_process",
    "try_host_process_exit_value",
    "destroy_host_process",
    "open_host_zip",
    "zip_entry_count",
    "zip_get_entry_info",
    "zip_read_entry",
    "open_host_byte_buffer",
    "bind_server_socket",
    "accept_connection",
    "connect_socket",
    "server_socket_local_port",
]

for method in methods_to_replace:
    interpreter_content = interpreter_content.replace(f"heap.{method}(", f"heap.host.{method}(")

with open('crates/duke-interpreter/src/lib.rs', 'w') as f:
    f.write(interpreter_content)

print("Done")
