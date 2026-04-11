import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    content = f.read()

# I see the problem. When a thread joins itself, `join_java_thread` calls:
# let is_finished = runtime.threads.records()...
# if it is itself, `is_finished` is FALSE!
# So it does: `runtime.handles.remove(&thread_id)`.
# Then `handle.join()`.
# But wait, does it get a handle?
# If the parent thread HAS NOT YET inserted the handle, `handle` is `None`!
# So it loops and yields.
# Parent thread is ALSO running. Wait, parent thread is `main`.
# `main` calls `spawn_java_thread`, gets the `thread_id`, and then calls `runtime.handles.insert(thread_id, handle)`.
# So `main` DOES insert the handle.
# Then `main` continues, and maybe it calls `t1.join()`.
# When `main` calls `t1.join()`, it calls `join_java_thread` for `t1` (Thread B).
# At this point, Thread B has ALSO called `t1.join()`, and is looping.
# Thread B takes its handle from `runtime.handles`, and calls `.join()`.
# BUT Thread B IS `handle`!
# Wait, when Thread B calls `.join()`, Rust's standard library blocks.
# Ah, `std::thread::JoinHandle::join()` will PANIC if called from the same thread: "thread joined itself".
# So Thread B panics!
# The panic unwinds Thread B.
# `run_thread_to_completion` never finishes, and the thread dies.
# But wait! When a thread dies due to panic, its `JoinHandle::join()` method (if called by SOMEONE ELSE) will return `Err(Box<dyn Any>)`.
# So now `main` thread is waiting on `t1.join()`.
# BUT Thread B already removed `t1`'s handle from `runtime.handles`!
# So when `main` calls `join_java_thread`, it finds `is_finished == false` (because Thread B died but never updated `is_finished`).
# And it finds `handle == None` (because Thread B removed it).
# So `main` yields in a `loop { ... std::thread::yield_now(); }` FOREVER!
# Deadlock!

# To fix this, we should NOT remove the handle unless we know we are NOT joining ourselves!
# Wait, even better: we shouldn't allow joining ourselves at all, or we shouldn't remove the handle.
# If we remove the handle, who puts it back? Nobody.
# The reason it removes the handle is because `std::thread::JoinHandle::join` takes `self` by value!
# If it's stored in a `HashMap`, we have to remove it to call `.join()`.
# But if it's the SAME thread, it shouldn't even attempt to join itself.
# We should probably change `NativeThreadAction::Join { thread_id }` to also pass the current `thread_id`?
# But `run_execution` doesn't know its own `thread_id` easily.
# Actually, if a thread panics, `live_workers` might not be decremented either!
# But `live_workers` is decremented in `spawn_java_thread`'s `handle = std::thread::spawn(move || { ... result })`.
# Wait, if `run_thread_to_completion` panics, the closure in `thread::spawn` will ALSO panic and unwind, so the code AFTER it (`live_workers.saturating_sub(1)`) WILL NEVER RUN!
# This means `live_workers` stays > 0 forever!
# AND `mark_finished_by_java_ref` never runs!
# So `is_finished` stays false forever!
# Any other thread that tries to join it will loop forever!
# We need to catch panics inside the thread, or use a `Drop` guard to clean up `live_workers` and `mark_finished`!

# But wait, there is no way a Java program is supposed to cause a Rust panic.
# It caused a Rust panic because it joined itself. We should PREVENT it from joining itself by returning a JVM error or something? Or just doing nothing (Java `Thread.join()` on itself blocks forever until interrupted).
# But wait, if Java blocks forever, that IS a deadlock. But the prompt says "fix the vulnerable code", which means the VM shouldn't crash or panic.
# If a Java thread joins itself, does it deadlock in Java?
# Yes, `Thread.currentThread().join()` deadlocks in Java forever!
# But it shouldn't cause a Rust panic or crash the VM in a bad way (e.g. poisoning).
# So we need to handle the case where `join_java_thread` doesn't remove the handle if it's currently running.
# Wait, `std::thread::current().id()` can be compared with the handle's thread ID!
# `handle.thread().id() == std::thread::current().id()`
# If they are equal, it's joining itself! We can just loop forever without calling `.join()`!
# OR we can just return a deadlock exception? Java allows deadlocks. So we should just loop and yield.

# Let's fix the Rust panic first:
content_new = content.replace(
"""        let handle = {
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
            runtime.handles.remove(&thread_id)
        };

        if let Some(handle) = handle {
            return match handle.join() {
                Ok(result) => result,
                Err(payload) => std::panic::resume_unwind(payload),
            };
        }""",
"""        let handle = {
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
                None
            } else {
                runtime.handles.remove(&thread_id)
            }
        };

        if let Some(handle) = handle {
            return match handle.join() {
                Ok(result) => result,
                Err(payload) => std::panic::resume_unwind(payload),
            };
        }""")

with open("crates/duke-interpreter/src/lib.rs", "w") as f:
    f.write(content_new)
