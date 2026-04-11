import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    content = f.read()

# I want to fail with an error or panic instead of hanging in an infinite loop!
# Wait, `Thread.currentThread().join()` in standard Java causes a deadlock. Should we return an error instead in duke-interpreter?
# The user's prompt specifically mentions hunting for "deadlocks". So a deadlock is a vulnerability I've found.
# How do I fix the deadlock? Throw an exception? Or return a `VmError`?
# In `std::thread`, joining yourself panics. I can return `Err(VmError::Unimplemented { mnemonic: "deadlock: thread joined itself" })`.
# Let's do that!

content_new = content.replace(
"""            // Check if we are trying to join ourselves to avoid Rust panic
            let is_self = if let Some(h) = runtime.handles.get(&thread_id) {
                h.thread().id() == std::thread::current().id()
            } else {
                false
            };

            if is_self {
                None
            } else {
                runtime.handles.remove(&thread_id)
            }""",
"""            // Check if we are trying to join ourselves to avoid Rust panic
            let is_self = if let Some(h) = runtime.handles.get(&thread_id) {
                h.thread().id() == std::thread::current().id()
            } else {
                false
            };

            if is_self {
                return Err(crate::error::VmError::Unimplemented {
                    mnemonic: "deadlock: thread joined itself",
                });
            } else {
                runtime.handles.remove(&thread_id)
            }""")

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.write(content_new)
