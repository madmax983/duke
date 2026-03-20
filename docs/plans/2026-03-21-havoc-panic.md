1. The Weak Point:
In `crates/duke-interpreter/src/lib.rs`, the `execute_class_to_completion` function relies on `wait_for_all_java_threads`.

If `wait_for_all_java_threads` encounters an error while joining threads (e.g., `result?`), it immediately returns the `VmError`, returning control to `execute_class_to_completion`.

```rust
fn wait_for_all_java_threads(
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
) -> VmResult<()> {
    loop {
...
        for handle in handles {
            match handle.join() {
                Ok(result) => result?, // <-- EARLY RETURN
                Err(payload) => std::panic::resume_unwind(payload),
            }
        }
    }
}
```

Then `execute_class_to_completion` attempts to unwrap the `shared` `Arc`:

```rust
    let wait_result = wait_for_all_java_threads(&runtime);
    let Ok(shared) = std::sync::Arc::try_unwrap(shared) else {
        panic!("completion runtime released shared VM state")
    };
```

If one thread crashed and exited early, but another thread was still running (e.g. executing `Thread.sleep()`), the remaining thread still holds a reference to `shared`. Thus `Arc::try_unwrap` fails and panics!

2. The Solution:
We need to ensure all threads finish executing, even if one fails. Or we change how we return the result. Or we just keep track of the first error encountered, but STILL join all other threads before returning.

Wait, if we change `wait_for_all_java_threads` to collect the first error, but keep joining the other threads...
But if a thread goes into an infinite loop and we don't force quit? The JVM continues running non-daemon threads until they finish. If an exception occurs in one thread, it dies, but the rest keep running!
Currently `duke` terminates the whole VM when any thread returns an error (since it returns `VmError`), but we still need to join everything else... wait, no. In Java, if a thread throws an uncaught exception, only that thread dies. The JVM keeps running.

Here, `wait_for_all_java_threads` does `result?`. This means if any thread returns an error, the VM panics when it tries to clean up because it doesn't wait for other threads to finish, causing `Arc::try_unwrap` to panic.

Let's modify `wait_for_all_java_threads` to wait for ALL handles in the current batch before returning the first error it saw.
Actually, if a thread dies, it shouldn't crash the whole VM unless it's the main thread. But `duke`'s current model seems to just bubble up `VmError`.
Let's just fix the panic: wait for all threads to finish.

```rust
fn wait_for_all_java_threads(
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
) -> VmResult<()> {
    let mut first_error = None;
    loop {
        let handles = {
            let mut runtime = runtime.lock().unwrap();
            if runtime.handles.is_empty() {
                break;
            }
            runtime
                .handles
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };

        for handle in handles {
            match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    if first_error.is_none() {
                        first_error = Some(err);
                    }
                }
                Err(payload) => std::panic::resume_unwind(payload),
            }
        }
    }

    if let Some(err) = first_error {
        Err(err)
    } else {
        Ok(())
    }
}
```
Wait, if `wait_for_all_java_threads` still returns an error, and other spawned threads spawned MORE threads before exiting, the loop continues and waits for them. Thus when `handles.is_empty()` is true, ALL threads are dead.
Then `Arc::try_unwrap` will succeed because there are no more threads holding the `Arc`.
