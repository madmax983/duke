1.  **🧨 IDENTIFY - The Weak Point:**
    *   There is a classic TOCTOU (Time-of-Check to Time-of-Use) race condition in `crates/duke-interpreter/src/native.rs` when interrupting a Java thread.
    *   The `native_thread_interrupt` function reads the thread ID from `java_thread_hosts()` via `host_thread_for_java_thread`, and *then* inserts it into `interrupted_host_threads()` via `interrupt_host_thread`.
    *   If a thread completes execution, `unregister_java_host_thread` will acquire the `java_thread_hosts` write lock, remove the thread ID, and then acquire the `interrupted_host_threads` write lock to remove it from the interrupted set.
    *   If the thread running `native_thread_interrupt` reads the thread ID from `java_thread_hosts`, then gets preempted before writing to `interrupted_host_threads`, and the target thread finishes and unregisters, it clears its interruption state (which is currently empty). Then, the interrupter resumes and writes to `interrupted_host_threads`.
    *   This leaves a "zombie" thread ID in the `interrupted_host_threads` set, leading to memory leaks over time, or worse, if `std::thread::ThreadId` is ever reused by the OS/Runtime (though Rust's `ThreadId` tries to be globally unique for the life of the process, a leak is a bug). We can prove this race condition with `loom`.

2.  **🔨 ATTACK - The Harness:**
    *   I have already written `tests/havoc_thread_state_loom.rs` which demonstrates this exact TOCTOU in a simplified, isolated test using `loom`. The test successfully failed with: `"TOCTOU race condition! Zombie thread ID left in interrupted set."`
    *   I need to incorporate this or a similar test into the `crates/duke-interpreter/tests` directory as `havoc_thread_interrupt_toctou.rs` (or similar) to follow Havoc's rules. But wait, `native.rs` has a bunch of internal private stuff. The best way to test the exact state is to create a Loom test that simulates the structure of the two maps. Let's make sure the test is fully green after fixing it.

3.  **🟢 FIX & REFACTOR:**
    *   Combine `java_thread_hosts` and `interrupted_host_threads` into a single `RwLock` or `Mutex` over a combined struct so the read-then-write of interrupt, and the write-then-write of unregister, happen atomically without dropping the lock in between.
    *   Wait, the context says: "When resolving a Time-of-Check to Time-of-Use (TOCTOU) race condition by combining multiple RwLocks into a single struct, ensure that dependent read-then-write operations (like retrieving an ID from one map to insert into another) are also combined into a single atomic operation under one write guard. Dropping a read guard to acquire a write guard reintroduces the race condition."
    *   I will refactor `crates/duke-interpreter/src/native.rs` to use a `ThreadRegistry` struct containing both `hosts` and `interrupted`, protected by a single `RwLock`.

4.  **🎁 PRESENT - The Wreckage:**
    *   Title: `👺 Havoc: TOCTOU in thread interruption leaves zombie ThreadIds`
    *   Description with Trigger, Stack Trace/Failure, Reproduction, and Comment.
