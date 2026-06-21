#![allow(missing_docs)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// Simulate the fixed implementation: a single lock protecting both collections
#[derive(Default)]
struct ThreadHostState {
    hosts: HashMap<i32, loom::thread::ThreadId>,
    interrupted: HashSet<loom::thread::ThreadId>,
}

#[test]
fn test_thread_interrupt_state_leak() {
    loom::model(|| {
        let state = Arc::new(RwLock::new(ThreadHostState::default()));

        let host_key = 1;
        let thread_id = loom::thread::current().id();
        state.write().unwrap().hosts.insert(host_key, thread_id);

        let s1 = state.clone();
        let t1 = thread::spawn(move || {
            // Thread A: interrupt
            // 1. Get lock, check, use - all inside the SAME write lock
            let mut state_lock = s1.write().unwrap();
            if let Some(&host_thread_id) = state_lock.hosts.get(&host_key) {
                state_lock.interrupted.insert(host_thread_id);
            }
        });

        let s2 = state.clone();
        let t2 = thread::spawn(move || {
            // Thread B: unregister
            let mut state_lock = s2.write().unwrap();
            let removed = state_lock.hosts.remove(&host_key);
            if let Some(id) = removed {
                state_lock.interrupted.remove(&id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // If the leak occurs, the interrupted set won't be empty!
        let is_empty = state.read().unwrap().interrupted.is_empty();
        assert!(is_empty, "State leak detected: interrupted thread ID was left behind!");
    });
}
