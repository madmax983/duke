1. We identified a critical system fragility in the `duke-interpreter` crate where multiple locks (`RwLock` for `ZIP_FILES`, `Mutex` for `SYSTEM_PROPERTY_OVERRIDES`, and `CompletionRuntime`) used `.unwrap()` blindly.
2. If any Java thread (or a bug in the VM execution layer) panicked while holding the lock, the lock was "poisoned".
3. All subsequent native method calls on those locks `.unwrap()` the PoisonError, resulting in a cascading panic that immediately brings down the entire Rust process (instead of propagating a recoverable error to the VM).
4. We simulated this vulnerability and proved it using standard `std::sync` primitives or `loom`.
5. We replaced these dangerous unwraps in `native.rs` with `.unwrap_or_else(|poisoned| poisoned.into_inner())` to gracefully recover the lock data and continue execution even if a panic occurred previously.
6. We replaced `.unwrap()` for `heap.get_mut(r)` inside value boxing methods with safe `?` operators or `if let Ok(obj) = heap.get_mut(r)`.
7. Pre-commit check to ensure proper testing, verification, review, and reflection are done.
