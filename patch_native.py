import re

with open('crates/duke-interpreter/src/native.rs', 'r') as f:
    content = f.read()

search = """        if let Some(handle) = handle {
            return match handle.join() {
                Ok(result) => result,
                Err(payload) => std::panic::resume_unwind(payload),
            };
        }"""

replace = """        if let Some(handle) = handle {
            if handle.thread().id() == std::thread::current().id() {
                // To avoid a Rust panic (`thread joined itself`), we restore the handle
                // and park indefinitely to mirror genuine JVM deadlock behavior.
                runtime.lock().unwrap().handles.insert(thread_id, handle);
                loop {
                    std::thread::park();
                }
            }
            return match handle.join() {
                Ok(result) => result,
                Err(payload) => std::panic::resume_unwind(payload),
            };
        }"""

if search in content:
    with open('crates/duke-interpreter/src/native.rs', 'w') as f:
        f.write(content.replace(search, replace))
    print("Patched!")
else:
    print("Search string not found")
