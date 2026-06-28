#![allow(missing_docs)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

struct JavaHostState {
    hosts: HashMap<i32, loom::thread::ThreadId>,
    interrupted: HashSet<loom::thread::ThreadId>,
}

#[test]
fn test_thread_host_state_leak() {
    loom::model(|| {
        let state = Arc::new(RwLock::new(JavaHostState {
            hosts: HashMap::new(),
            interrupted: HashSet::new(),
        }));

        // Thread A registers a host thread and then gets its thread ID.
        let state1 = state.clone();
        let t1 = thread::spawn(move || {
            let host_key = 1;
            let thread_id = loom::thread::current().id();
            state1.write().unwrap().hosts.insert(host_key, thread_id);

            // Interrupt thread
            let mut w = state1.write().unwrap();
            if let Some(id) = w.hosts.get(&host_key).copied() {
                w.interrupted.insert(id);
            }
        });

        // Thread B unregisters the thread.
        let state2 = state.clone();
        let t2 = thread::spawn(move || {
            let host_key = 1;
            let mut w = state2.write().unwrap();
            let removed = w.hosts.remove(&host_key);
            if let Some(id) = removed {
                w.interrupted.remove(&id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();
                let final_state = state.read().unwrap();
        for id in &final_state.interrupted {
            assert!(final_state.hosts.values().any(|v| v == id), "Memory leak detected!");
        }
    });
}
