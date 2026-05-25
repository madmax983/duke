#![allow(missing_docs)]
#![allow(clippy::significant_drop_tightening)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[test]
fn test_interrupted_thread_leak_race() {
    loom::model(|| {
        let hosts = Arc::new(RwLock::new(HashMap::<i32, loom::thread::ThreadId>::new()));
        let interrupted = Arc::new(RwLock::new(HashSet::<loom::thread::ThreadId>::new()));

        let target_id = loom::thread::current().id();
        hosts.write().unwrap().insert(1, target_id);

        let h1 = hosts.clone();
        let i1 = interrupted.clone();
        // Thread 1: unregister
        let t1 = thread::spawn(move || {
            let mut hosts_write = h1.write().unwrap();
            let removed = hosts_write.remove(&1);
            if let Some(id) = removed {
                i1.write().unwrap().remove(&id);
            }
        });

        let h2 = hosts;
        let i2 = interrupted.clone();
        // Thread 2: interrupt
        let t2 = thread::spawn(move || {
            let hosts_read = h2.read().unwrap();
            let id = hosts_read.get(&1).copied();
            if let Some(id) = id {
                i2.write().unwrap().insert(id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        assert!(
            interrupted.read().unwrap().is_empty(),
            "Leak detected: thread ID remained in interrupted set after unregistration"
        );
    });
}
