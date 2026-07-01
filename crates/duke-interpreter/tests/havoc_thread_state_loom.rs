#![allow(missing_docs)]
use std::sync::Arc;
use std::collections::{HashMap, HashSet};

struct ThreadState {
    hosts: HashMap<i32, usize>,
    interrupted: HashSet<usize>,
}

#[test]
fn test_thread_state_toctou() {
    loom::model(|| {
        let state = Arc::new(loom::sync::RwLock::new(ThreadState {
            hosts: HashMap::new(),
            interrupted: HashSet::new(),
        }));

        state.write().unwrap().hosts.insert(1, 100);

        let s1 = state.clone();
        let t1 = loom::thread::spawn(move || {
            // New atomic logic
            let mut w = s1.write().unwrap();
            if let Some(id) = w.hosts.get(&1).copied() {
                w.interrupted.insert(id);
            }
        });

        let s2 = state.clone();
        let t2 = loom::thread::spawn(move || {
            let mut w = s2.write().unwrap();
            if let Some(id) = w.hosts.remove(&1) {
                w.interrupted.remove(&id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        assert!(
            state.read().unwrap().interrupted.is_empty(),
            "Memory leak: interrupted set is not empty! Host thread was unmapped but later marked as interrupted."
        );
    });
}
