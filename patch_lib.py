import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    content = f.read()

content_new = content.replace(
"""    let handle = std::thread::spawn(move || {
        let result = run_thread_to_completion(state, &shared_clone, &runtime_clone, &loader_clone);
        {
            let mut shared = shared_clone.lock().unwrap();
            shared.live_workers = shared.live_workers.saturating_sub(1);
        }
        let _ = runtime_clone
            .lock()
            .unwrap()
            .threads
            .mark_finished_by_java_ref(thread_ref);
        result
    });""",
"""    let handle = std::thread::spawn(move || {
        let my_id = std::thread::current().id();
        runtime_clone.lock().unwrap().threads.set_rust_thread_id(thread_ref, my_id);

        let result = run_thread_to_completion(state, &shared_clone, &runtime_clone, &loader_clone);
        {
            let mut shared = shared_clone.lock().unwrap();
            shared.live_workers = shared.live_workers.saturating_sub(1);
        }
        let _ = runtime_clone
            .lock()
            .unwrap()
            .threads
            .mark_finished_by_java_ref(thread_ref);
        result
    });""")

content_new = content_new.replace(
"""fn join_java_thread(
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    thread_id: i32,
) -> VmResult<()> {
    println!("thread joining {}", thread_id);
    loop {
        let handle = {
            let mut runtime = runtime.lock().unwrap();
            let is_finished = runtime
                .threads
                .records()
                .iter()
                .find(|record| record.thread_id == thread_id)
                .is_none_or(|record| record.finished);
            if is_finished {
                return Ok(());
            }

            // Check if we are trying to join ourselves to avoid Rust panic
            let is_self = if let Some(h) = runtime.handles.get(&thread_id) {
                h.thread().id() == std::thread::current().id()
            } else {
                false
            };

            if is_self {
                println!("DEADLOCK DETECTED");
                return Err(VmError::Unimplemented {
                    mnemonic: "deadlock: thread joined itself",
                });
            } else {
                runtime.handles.remove(&thread_id)
            }
        };

        if let Some(handle) = handle {
            return match handle.join() {
                Ok(result) => result,
                Err(payload) => std::panic::resume_unwind(payload),
            };
        }

        std::thread::yield_now();
    }
}""",
"""fn join_java_thread(
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    thread_id: i32,
) -> VmResult<()> {
    loop {
        let handle = {
            let mut runtime = runtime.lock().unwrap();

            let record = runtime
                .threads
                .records()
                .iter()
                .find(|record| record.thread_id == thread_id);

            let is_finished = record.is_none_or(|r| r.finished);
            if is_finished {
                return Ok(());
            }

            // Avoid joining ourselves
            if let Some(rec) = record {
                if rec.rust_thread_id == Some(std::thread::current().id()) {
                    return Err(VmError::Unimplemented {
                        mnemonic: "deadlock: thread joined itself",
                    });
                }
            }

            runtime.handles.remove(&thread_id)
        };

        if let Some(handle) = handle {
            return match handle.join() {
                Ok(result) => result,
                Err(payload) => std::panic::resume_unwind(payload),
            };
        }

        std::thread::yield_now();
    }
}""")

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.write(content_new)
