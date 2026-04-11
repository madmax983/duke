import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    content = f.read()

content_new = content.replace(
"""            if let Some(rec) = record {
                if rec.rust_thread_id == Some(std::thread::current().id()) {
                    return Err(VmError::Unimplemented {
                        mnemonic: "deadlock: thread joined itself",
                    });
                }
            }""",
"""            if let Some(rec) = record
                && rec.rust_thread_id == Some(std::thread::current().id())
            {
                return Err(VmError::Unimplemented {
                    mnemonic: "deadlock: thread joined itself",
                });
            }""")

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.write(content_new)
