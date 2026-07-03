#![allow(missing_docs)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[test]
fn test_thread_registry_toctou_leak() {
    loom::model(|| {
        #[derive(Default)]
        struct HostThreadsState {
            hosts: HashMap<i32, loom::thread::ThreadId>,
            interrupted: HashSet<loom::thread::ThreadId>,
        }

        let state = Arc::new(RwLock::new(HostThreadsState::default()));

        let target_thread_id = loom::thread::current().id();
        state.write().unwrap().hosts.insert(42, target_thread_id);

        let t1_state = state.clone();

        let t1 = thread::spawn(move || {
            // Simulate native_thread_interrupt (Fixed TOCTOU Code)
            let mut s = t1_state.write().unwrap();
            let host_thread_id = s.hosts.get(&42).copied();
            if let Some(id) = host_thread_id {
                s.interrupted.insert(id);
            }
        });

        let t2_state = state.clone();

        let t2 = thread::spawn(move || {
            // Simulate unregister_java_host_thread (New Code)
            // Acquires the lock once and modifies both.
            let mut s = t2_state.write().unwrap();
            let removed = s.hosts.remove(&42);
            if let Some(id) = removed {
                s.interrupted.remove(&id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let is_empty = state.read().unwrap().interrupted.is_empty();
        assert!(
            is_empty,
            "Memory leak: interrupted_host_threads still contains the unregistered thread id!"
        );
    });
}
