#![allow(missing_docs)]
#![allow(clippy::significant_drop_tightening)]
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

// In this test, we simulate the `unwrap()` in `native_thread_start`.
// We use std::thread directly so that the panic from thread 1 can be isolated,
// and then we verify thread 2 doesn't crash when it hits unwrap_or_else.

#[test]
fn test_thread_start_panic() {
    let runtime = Arc::new(Mutex::new(0));

    let r1 = runtime.clone();
    let t1 = thread::spawn(move || {
        let mut g = r1.lock().unwrap();
        *g += 1;
        panic!("Poisoning the mutex!");
    });

    let _ = t1.join(); // Ignore error

    let r2 = runtime;
    let t2 = thread::spawn(move || {
        // Using into_inner safely acquires the lock even if poisoned.
        let mut g = r2.lock().unwrap_or_else(|e| e.into_inner());
        *g += 1;
    });

    t2.join().unwrap(); // Should not panic!
}
