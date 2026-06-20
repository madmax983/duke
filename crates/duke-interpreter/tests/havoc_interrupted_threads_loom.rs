#![allow(missing_docs)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

struct HostThreadState {
    hosts: HashMap<i32, loom::thread::ThreadId>,
    interrupted: HashSet<loom::thread::ThreadId>,
}

#[test]
fn test_interrupted_threads_leak() {
    loom::model(|| {
        let state = Arc::new(RwLock::new(HostThreadState {
            hosts: HashMap::new(),
            interrupted: HashSet::new(),
        }));

        let state1 = state.clone();
        let t1 = thread::spawn(move || {
            // Simulating the thread that registers and unregisters
            let id = loom::thread::current().id();
            state1.write().unwrap().hosts.insert(1, id);

            // Simulate unregister_java_host_thread atomicity
            let mut s = state1.write().unwrap();
            let removed = s.hosts.remove(&1);
            if let Some(tid) = removed {
                s.interrupted.remove(&tid);
            }
        });

        let state2 = state.clone();
        let t2 = thread::spawn(move || {
            // Simulating native_thread_interrupt atomicity
            let mut s = state2.write().unwrap();
            let id_opt = s.hosts.get(&1).copied();
            if let Some(tid) = id_opt {
                s.interrupted.insert(tid);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // If the thread has unregistered, it should NOT be in the interrupted set
        assert!(state.read().unwrap().interrupted.is_empty(), "Thread ID leaked in interrupted_host_threads!");
    });
}
