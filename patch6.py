import re

with open("crates/duke-interpreter/src/lib.rs", "r") as f:
    content = f.read()

# "thread joining 0"
# Thread 0 is the main thread!
# wait, `main` thread id in ThreadRecord?
# Let's check how the main thread gets a `thread_id`.
# The main thread doesn't have an entry in `runtime.handles`!
# `execute_class_to_completion` sets up `vm` and `runtime` and then loops `run_thread_to_completion`-like manually:
# "let run_result: VmResult<Option<Slot>> = loop { ... }"
# It's NOT spawned via `std::thread::spawn`! It runs on the calling thread.
# So `runtime.handles` will NEVER contain the handle for `thread_id = 0` (or whatever ID it gets, wait, does it even get an ID?)
# The main thread doesn't even have a `ThreadRecord` registered in `runtime.threads`!
# Oh. If the main thread calls `this.join()`, it's joining itself, but its ID isn't 0.
# Wait, `native_thread_join` uses `thread.fields[THREAD_ID_SLOT]`.
# How did `ThreadBug.main` get `t1.join()` to mean thread 0?
# Wait!
# In `ThreadBug.java`:
#   MyThread t = new MyThread();
#   t.start();
#   t.join();
# Is `t.join()` joining thread 0?
# When `new MyThread()` is executed, it does `Thread.<init>`.
# Does `Thread.<init>` assign an ID? No, `spawn_java_thread` assigns the ID!
# `spawn_java_thread` does `thread_id = runtime.threads.allocate_thread_id()`.
# The first allocated ID is `0`.
# So `t1` gets ID `0`.
# Then `t1` starts executing. It prints "thread joining 0".
# It calls `this.join()`. `this` is `t1` (ID `0`).
# So `t1` (the spawned thread) calls `join_java_thread(..., 0)`.
# Since it is `t1` calling `join` on `0`, we should see `DEADLOCK DETECTED`!
# BUT `runtime.handles` might NOT contain the handle yet!
# The main thread (parent) called `t.start()`, which spawned the thread and returned to interpreter.
# Wait, `spawn_java_thread` DOES insert the handle before returning!
# `runtime.lock().unwrap().handles.insert(thread_id, handle); Ok(())`.
# So the handle IS in `runtime.handles` by the time `spawn_java_thread` returns.
# BUT wait! What if `main` finishes, and THEN `t1` tries to join itself?
# `main` calls `t.join()`. `main` calls `join_java_thread(..., 0)`.
# So `main` ALSO prints "thread joining 0".
# Let's check the test output:
# thread joining 0
# thread joining 0
# TWO threads are trying to join thread 0!
# One is `main` thread, one is `t1` thread!
# Which one gets the handle from `runtime.handles.remove(&thread_id)`?
# Let's say `main` calls it first. `main` gets the handle, and calls `handle.join()`.
# `main` is now blocked on `t1` finishing.
# Then `t1` reaches `this.join()`.
# `t1` calls `join_java_thread(..., 0)`.
# But `runtime.handles` is NOW EMPTY for `0` because `main` removed it!
# So `t1` finds `handle = None`.
# `t1` calls `std::thread::yield_now()` and loops!
# This is why `t1` hangs forever!
# `t1` is waiting for itself to finish, but `main` already took the handle. So `t1` yields forever.
# `main` is waiting for `t1` to finish.
# Deadlock!

# How to fix:
# `join_java_thread` should NOT loop forever if it's joining itself, EVEN IF the handle is already gone!
# We need to know the current thread's ID.
# But we can't easily know the current Java thread ID, unless we look at the Rust thread ID.
# But wait, `ThreadRecord` could store the Rust thread ID!
# `std::thread::current().id()` gives the Rust thread ID!
# But `ThreadRecord` is created before the thread starts executing.
# Actually, when `t1` calls `yield_now()`, it means `handle` is `None` AND `is_finished` is false.
# Why is `handle` `None`? Because another thread is already joining it!
# In Java, multiple threads can join the same thread.
# If multiple threads call `join()`, they should all block until the thread is finished.
# Our current implementation:
# Thread A removes the handle and calls `handle.join()`. Thread A blocks.
# Thread B removes... it's `None`. Thread B calls `yield_now()`. Thread B busy-waits!
# That's bad. Multiple threads joining causes busy-waiting for the others.
# But if a thread joins ITSELF, and another thread ALSO joins it, the thread itself busy-waits!
# If it busy-waits, it never finishes. So the other thread waits forever.
# Deadlock!

# To fix this, ANY thread joining ITSELF should immediately error out or return!
# How do we know if it's joining itself?
# We can't rely on `runtime.handles` because another thread might have removed it.
# We need to store the `std::thread::ThreadId` in `ThreadRecord`?
# But `ThreadRecord` is created BEFORE `std::thread::spawn`. We don't know the `ThreadId` yet!
# We can update it inside the spawned thread!
# `let current_id = std::thread::current().id();`
# In `spawn_java_thread` closure:
# `runtime_clone.lock().unwrap().threads.set_rust_thread_id(thread_ref, current_id);`
# Then `join_java_thread` can just check:
# `if record.rust_thread_id == Some(std::thread::current().id()) { return Err(...); }`

content_new = content.replace(
"""pub struct ThreadRecord {
    pub java_ref: u64,
    pub thread_id: i32,
    pub finished: bool,
    pub daemon: bool,
}

impl ThreadRecord {
    #[must_use]
    pub const fn new(java_ref: u64, thread_id: i32) -> Self {
        Self {
            java_ref,
            thread_id,
            finished: false,
            daemon: false,
        }
    }""",
"""pub struct ThreadRecord {
    pub java_ref: u64,
    pub thread_id: i32,
    pub finished: bool,
    pub daemon: bool,
    pub rust_thread_id: Option<std::thread::ThreadId>,
}

impl ThreadRecord {
    #[must_use]
    pub const fn new(java_ref: u64, thread_id: i32) -> Self {
        Self {
            java_ref,
            thread_id,
            finished: false,
            daemon: false,
            rust_thread_id: None,
        }
    }""")

content_new = content_new.replace(
"""    pub fn mark_finished_by_java_ref(&mut self, java_ref: u64) -> bool {
        let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.java_ref == java_ref)
        else {
            return false;
        };
        record.mark_finished()
    }""",
"""    pub fn mark_finished_by_java_ref(&mut self, java_ref: u64) -> bool {
        let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.java_ref == java_ref)
        else {
            return false;
        };
        record.mark_finished()
    }

    pub fn set_rust_thread_id(&mut self, java_ref: u64, id: std::thread::ThreadId) {
        if let Some(record) = self.records.iter_mut().find(|r| r.java_ref == java_ref) {
            record.rust_thread_id = Some(id);
        }
    }""")

with open("crates/duke-interpreter/src/threading.rs", "w") as f:
    # First apply to threading.rs
    with open("crates/duke-interpreter/src/threading.rs", "r") as f_read:
        threading_content = f_read.read()

    threading_content = threading_content.replace(
"""pub struct ThreadRecord {
    pub java_ref: u64,
    pub thread_id: i32,
    pub finished: bool,
    pub daemon: bool,
}

impl ThreadRecord {
    #[must_use]
    pub const fn new(java_ref: u64, thread_id: i32) -> Self {
        Self {
            java_ref,
            thread_id,
            finished: false,
            daemon: false,
        }
    }""",
"""pub struct ThreadRecord {
    pub java_ref: u64,
    pub thread_id: i32,
    pub finished: bool,
    pub daemon: bool,
    pub rust_thread_id: Option<std::thread::ThreadId>,
}

impl ThreadRecord {
    #[must_use]
    pub const fn new(java_ref: u64, thread_id: i32) -> Self {
        Self {
            java_ref,
            thread_id,
            finished: false,
            daemon: false,
            rust_thread_id: None,
        }
    }""")

    threading_content = threading_content.replace(
"""    pub fn mark_finished_by_java_ref(&mut self, java_ref: u64) -> bool {
        let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.java_ref == java_ref)
        else {
            return false;
        };
        record.mark_finished()
    }""",
"""    pub fn mark_finished_by_java_ref(&mut self, java_ref: u64) -> bool {
        let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.java_ref == java_ref)
        else {
            return false;
        };
        record.mark_finished()
    }

    pub fn set_rust_thread_id(&mut self, java_ref: u64, id: std::thread::ThreadId) {
        if let Some(record) = self.records.iter_mut().find(|r| r.java_ref == java_ref) {
            record.rust_thread_id = Some(id);
        }
    }""")

    f.write(threading_content)
