#![allow(missing_docs)]
use loom::sync::Arc;
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};

struct ThreadRegistry {
    hosts: HashMap<i32, usize>,
    interrupted: HashSet<usize>,
}

impl ThreadRegistry {
    fn new() -> Self {
        Self {
            hosts: HashMap::new(),
            interrupted: HashSet::new(),
        }
    }
}

#[test]
fn test_thread_state_toctou_fixed() {
    loom::model(|| {
        let registry = Arc::new(RwLock::new(ThreadRegistry::new()));

        registry.write().unwrap().hosts.insert(1, 100);

        let reg1 = registry.clone();
        let t1 = thread::spawn(move || {
            // Thread 1: Unregister thread
            let mut w = reg1.write().unwrap();
            if let Some(tid) = w.hosts.remove(&1) {
                w.interrupted.remove(&tid);
            }
        });

        let reg2 = registry.clone();
        let t2 = thread::spawn(move || {
            // Thread 2: Interrupt thread (FIXED LOGIC)
            // Retrieve ID and set flag under a single write lock.
            let mut w = reg2.write().unwrap();
            if let Some(tid) = w.hosts.get(&1).copied() {
                w.interrupted.insert(tid);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // After both threads finish, the state should be clean.
        assert!(
            registry.read().unwrap().interrupted.is_empty(),
            "TOCTOU race condition! Zombie thread ID left in interrupted set."
        );
    });
}
