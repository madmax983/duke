#![allow(missing_docs)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[test]
fn test_interrupted_threads_leak() {
    loom::model(|| {
        let hosts = Arc::new(RwLock::new(HashMap::<i32, usize>::new()));
        let interrupted = Arc::new(RwLock::new(HashSet::<usize>::new()));

        let h1 = hosts.clone();
        let i1 = interrupted.clone();

        // Initial state
        hosts.write().unwrap().insert(1, 100);

        let h2 = Arc::clone(&hosts);
        let i2 = interrupted.clone();

        // Thread A: interrupt
        let t1 = thread::spawn(move || {
            let host_thread_id = {
                let map = h1.read().unwrap();
                map.get(&1).copied()
            };
            if let Some(id) = host_thread_id {
                // FIXED logic
                let map = h1.read().unwrap();
                if map.get(&1) == Some(&id) {
                    i1.write().unwrap().insert(id);
                }
            }
        });

        // Thread B: unregister
        let t2 = thread::spawn(move || {
            let removed_host_thread = h2.write().unwrap().remove(&1);
            if let Some(id) = removed_host_thread {
                i2.write().unwrap().remove(&id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let leak = interrupted.read().unwrap().len();
        assert_eq!(leak, 0, "Memory leak detected in interrupted_host_threads!");
    });
}
