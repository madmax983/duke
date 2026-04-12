import re

with open('crates/duke-interpreter/src/native.rs', 'r') as f:
    content = f.read()

search = """            runtime
                .handles
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };

        for handle in handles {
            match handle.join() {
                Ok(result) => {
                    if let Err(e) = result {
                        first_error.get_or_insert(Err(e));
                    }
                }
                Err(payload) => std::panic::resume_unwind(payload),
            }
        }"""

replace = """            runtime
                .handles
                .drain()
                .collect::<Vec<_>>()
        };

        for (thread_id, handle) in handles {
            if handle.thread().id() == std::thread::current().id() {
                // Do not attempt to join ourselves during shutdown.
                // Restore the handle so it remains attached to the runtime context.
                runtime.lock().unwrap().handles.insert(thread_id, handle);
                continue;
            }
            match handle.join() {
                Ok(result) => {
                    if let Err(e) = result {
                        first_error.get_or_insert(Err(e));
                    }
                }
                Err(payload) => std::panic::resume_unwind(payload),
            }
        }"""

if search in content:
    with open('crates/duke-interpreter/src/native.rs', 'w') as f:
        f.write(content.replace(search, replace))
    print("Patched wait_for_all_java_threads!")
else:
    print("Search string not found in wait_for_all_java_threads")
