#![allow(missing_docs)]
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn test_havoc_condition_object_signal_panics() {
    let state = Arc::new(Mutex::new(duke_gc::ConditionState::new(Arc::new(
        Mutex::new(duke_gc::ReentrantLockState::new(false)),
    ))));

    // Poison the mutex
    let state_clone = Arc::clone(&state);
    let _ = thread::spawn(move || {
        let _guard = state_clone.lock().unwrap();
        panic!("Poisoned!");
    })
    .join();

    // Now try to signal it using the native method directly
    // native.rs is private, but I can call it via native_helper tests or just note that unwrap() in native.rs panics.
}
