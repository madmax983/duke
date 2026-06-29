use loom::sync::Arc;
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};

struct ThreadState {
    hosts: HashMap<i32, usize>,
    interrupted: HashSet<usize>,
}

#[test]
fn test_java_thread_interruption_race() {
    loom::model(|| {
        let state = Arc::new(RwLock::new(ThreadState {
            hosts: HashMap::new(),
            interrupted: HashSet::new(),
        }));

        state.write().unwrap().hosts.insert(1, 100);

        let state1 = state.clone();
        let t1 = thread::spawn(move || {
            let mut guard = state1.write().unwrap();
            if let Some(host_thread_id) = guard.hosts.remove(&1) {
                guard.interrupted.remove(&host_thread_id);
            }
        });

        let state2 = state.clone();
        let t2 = thread::spawn(move || {
            let mut guard = state2.write().unwrap();
            if let Some(id) = guard.hosts.get(&1).copied() {
                // Yield to test possible interleavings during write lock
                loom::thread::yield_now();
                guard.interrupted.insert(id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let is_empty = state.read().unwrap().interrupted.is_empty();
        assert!(is_empty, "Leak detected: interrupted set is not empty!");
    });
}
