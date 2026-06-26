#![allow(missing_docs)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

struct ThreadHostState {
    hosts: HashMap<i32, loom::thread::ThreadId>,
    interrupted: HashSet<loom::thread::ThreadId>,
}

#[test]
fn test_thread_state_leak() {
    loom::model(|| {
        let state = Arc::new(RwLock::new(ThreadHostState {
            hosts: HashMap::new(),
            interrupted: HashSet::new(),
        }));

        let target_thread_id = loom::thread::current().id();
        state.write().unwrap().hosts.insert(42, target_thread_id);

        let state_clone1 = state.clone();
        let t1 = thread::spawn(move || {
            // Simulate native_thread_interrupt (combines read and write into one lock context if needed, or single write lock)
            // The fix will lock `state` for reading (or writing) and then do the operation atomically.
            let mut s = state_clone1.write().unwrap();
            let host_thread_id_opt = s.hosts.get(&42).copied();
            if let Some(id) = host_thread_id_opt {
                s.interrupted.insert(id);
            }
        });

        let state_clone2 = state.clone();
        let t2 = thread::spawn(move || {
            // Simulate unregister_java_host_thread
            let mut s = state_clone2.write().unwrap();
            let removed = s.hosts.remove(&42);
            if let Some(id) = removed {
                s.interrupted.remove(&id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let s = state.read().unwrap();
        let is_hosts_empty = s.hosts.is_empty();
        let is_interrupted_empty = s.interrupted.is_empty();

        if is_hosts_empty && !is_interrupted_empty {
            panic!("State leak detected! ThreadId leaked in interrupted set.");
        }
    });
}
