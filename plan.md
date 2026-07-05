1. **Combine Locks to Avoid TOCTOU Race Condition**:
   - The TOCTOU (Time-of-Check to Time-of-Use) race condition in `interrupt_host_thread` happens because `host_thread_for_java_thread(host_key)` acquires a read lock on `java_thread_hosts()`, retrieves the thread ID, and then drops the read lock. It then calls `interrupt_host_thread(host_thread_id)` which acquires a write lock on `interrupted_host_threads()`.
   - In between dropping the read lock and acquiring the write lock, another thread could call `unregister_java_host_thread(host_key)`, which acquires the write lock on `java_thread_hosts()`, removes the thread ID, and then removes it from `interrupted_host_threads()`.
   - Then the first thread acquires the write lock on `interrupted_host_threads()` and inserts the thread ID. This causes a memory leak / state corruption because the `host_thread_id` is permanently stored in `interrupted_host_threads()` for a thread that is unregistered!
   - To fix this, I will combine the two `RwLock`s (`HOSTS` and `INTERRUPTED`) into a single `ThreadHostState` struct protected by a single `RwLock`.

2. **Refactor `native.rs`**:
   - Add a struct `ThreadHostState` that contains `hosts: HashMap<i32, std::thread::ThreadId>` and `interrupted: HashSet<std::thread::ThreadId>`.
   - Create a single `static THREAD_HOST_STATE: OnceLock<RwLock<ThreadHostState>> = OnceLock::new();` and helper function `thread_host_state()`.
   - Rewrite the associated functions: `register_java_host_thread`, `unregister_java_host_thread`, `host_thread_for_java_thread`, `java_host_key_for_current_host`, `current_host_thread_is_interrupted`, and `take_current_host_thread_interrupted`.
   - Create a unified `interrupt_java_host_thread(host_key: i32)` function which locks `THREAD_HOST_STATE` for writing once, looks up the `host_thread_id` from the map, and immediately inserts it into `interrupted`, without dropping the lock. This resolves the race condition.

3. **Update Callers**:
   - Find all places calling `interrupt_host_thread(host_thread_id)` and instead use the atomic `interrupt_java_host_thread(host_key)`.

4. **Verify the Fix**:
   - Create a loom test `tests/havoc_thread_unregister_race_loom.rs` that reproduces the issue with the `ThreadHostState` abstraction (with `loom::sync` versions).
   - Ensure `cargo test` passes.
   - Refactor codebase with Red-Green-Refactor to fix the test and make it green.

5. **Complete Pre Commit Steps**:
   - Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.

6. **Submit Code Review**:
   - Create a PR titled `👺 Havoc: [Thread State TOCTOU Race Condition]` following the Chaos persona instructions.
