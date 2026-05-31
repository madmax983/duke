#![allow(missing_docs)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// Simulate the TOCTOU race condition in interrupted_host_threads and java_thread_hosts interaction.
// In `native.rs`:
// `native_thread_is_interrupted` first checks if the thread is in `java_thread_hosts()`
// (which acquires and releases the read lock). Then, if found, it acquires the read lock
// on `interrupted_host_threads()` to check the interrupt status.
//
// Concurrently, `unregister_java_host_thread` can acquire the write lock on `java_thread_hosts()`
// and then acquire the write lock on `interrupted_host_threads()`, removing the thread.
// Because the reads are separate, `is_interrupted` might get a ThreadId, yield,
// and then the thread gets unregistered, meaning it checks a potentially stale ThreadId
// in the `interrupted` set.

#[test]
fn test_interrupted_threads_toctou() {
    loom::model(|| {
        let hosts = Arc::new(RwLock::new(HashMap::<i32, loom::thread::ThreadId>::new()));
        let interrupted = Arc::new(RwLock::new(HashSet::<loom::thread::ThreadId>::new()));

        let h1 = hosts.clone();
        let i1 = interrupted.clone();
        let t1 = thread::spawn(move || {
            // unregister_java_host_thread
            let removed = h1.write().unwrap().remove(&1);
            if let Some(id) = removed {
                i1.write().unwrap().remove(&id);
            }
        });

        let h2 = hosts.clone();
        let i2 = interrupted.clone();
        let t2 = thread::spawn(move || {
            // host_thread_for_java_thread
            if let Some(id) = h2.read().unwrap().get(&1).copied() {
                // native_thread_is_interrupted reading from interrupted_host_threads
                let _ = i2.read().unwrap().contains(&id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}
