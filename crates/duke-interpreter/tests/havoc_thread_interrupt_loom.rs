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
fn test_thread_interrupt_leak() {
    loom::model(|| {
        let state = Arc::new(RwLock::new(HostThreadState {
            hosts: HashMap::new(),
            interrupted: HashSet::new(),
        }));

        let host_key = 42;
        let mock_id = loom::thread::current().id();
        state.write().unwrap().hosts.insert(host_key, mock_id);

        let state1 = state.clone();

        let t1 = thread::spawn(move || {
            let mut s = state1.write().unwrap();
            if let Some(&host_thread_id) = s.hosts.get(&host_key) {
                s.interrupted.insert(host_thread_id);
            }
        });

        let state2 = state.clone();

        let t2 = thread::spawn(move || {
            let mut s = state2.write().unwrap();
            if let Some(host_thread_id) = s.hosts.remove(&host_key) {
                s.interrupted.remove(&host_thread_id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        assert!(
            state.read().unwrap().interrupted.is_empty(),
            "State leak detected: host thread ID was left in interrupted_host_threads"
        );
    });
}
