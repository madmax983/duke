import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    content = f.read()

# Let's add a println to see if it even reaches the check!
content_new = content.replace(
"""            if is_self {
                return Err(VmError::Unimplemented {
                    mnemonic: "deadlock: thread joined itself",
                });
            } else {
                runtime.handles.remove(&thread_id)
            }""",
"""            if is_self {
                println!("DEADLOCK DETECTED");
                return Err(VmError::Unimplemented {
                    mnemonic: "deadlock: thread joined itself",
                });
            } else {
                runtime.handles.remove(&thread_id)
            }""")

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.write(content_new)
