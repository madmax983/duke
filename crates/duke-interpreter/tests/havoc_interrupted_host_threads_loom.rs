#![allow(missing_docs)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Default)]
struct HostThreadState {
    hosts: HashMap<i32, loom::thread::ThreadId>,
    interrupted: HashSet<loom::thread::ThreadId>,
}

#[test]
fn test_host_thread_interruption_leak() {
    loom::model(|| {
        let state = Arc::new(RwLock::new(HostThreadState::default()));

        let target_thread = thread::current().id();
        let target_key = 42;

        state
            .write()
            .unwrap()
            .hosts
            .insert(target_key, target_thread);

        let state1 = state.clone();
        // Thread 1: simulate native_thread_interrupt
        let t1 = thread::spawn(move || {
            // Test the fixed logic: atomicity in interrupt_java_host_thread
            let mut s = state1.write().unwrap();
            if let Some(id) = s.hosts.get(&target_key).copied() {
                s.interrupted.insert(id);
            }
        });

        let state2 = state.clone();
        // Thread 2: simulate unregister_java_host_thread
        let t2 = thread::spawn(move || {
            let mut s = state2.write().unwrap();
            let removed = s.hosts.remove(&target_key);
            if let Some(id) = removed {
                s.interrupted.remove(&id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // If interrupted state is not empty, we leaked state!
        assert!(
            state.read().unwrap().interrupted.is_empty(),
            "Leaked interrupted thread state!"
        );
    });
}
