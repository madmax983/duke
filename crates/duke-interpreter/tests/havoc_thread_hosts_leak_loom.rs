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
fn test_thread_unregister_interrupt_race_fixed() {
    loom::model(|| {
        let state = Arc::new(RwLock::new(HostThreadState::default()));

        let main_id = thread::current().id();
        state.write().unwrap().hosts.insert(1, main_id);

        let state_t1 = state.clone();
        let t1 = thread::spawn(move || {
            // Unregister
            let mut w = state_t1.write().unwrap();
            let removed = w.hosts.remove(&1);
            if let Some(id) = removed {
                w.interrupted.remove(&id);
            }
        });

        let state_t2 = state.clone();
        let t2 = thread::spawn(move || {
            // Interrupt
            // Simulate the native_thread_is_interrupted / interrupt flow
            // Actually `interrupt_host_thread` just takes the id directly
            // But if we simulate taking it from `host_thread_for_java_thread` first:
            let mut w = state_t2.write().unwrap();
            if let Some(&id) = w.hosts.get(&1) {
                w.interrupted.insert(id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let leak = state.read().unwrap().interrupted.contains(&main_id);
        assert!(!leak, "Thread state leaked into interrupted_host_threads!");
    });
}
