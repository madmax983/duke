use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Default)]
struct JavaThreadState {
    hosts: HashMap<i32, usize>,
    interrupted: HashSet<usize>,
}

fn register_java_host_thread(
    state: &RwLock<JavaThreadState>,
    host_key: i32,
    host_thread_id: usize,
) {
    state
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .hosts
        .insert(host_key, host_thread_id);
}

fn unregister_java_host_thread(state: &RwLock<JavaThreadState>, host_key: i32) {
    let mut state = state
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(host_thread_id) = state.hosts.remove(&host_key) {
        state.interrupted.remove(&host_thread_id);
    }
}

fn interrupt_host_thread(state: &RwLock<JavaThreadState>, host_key: i32) {
    let mut state = state
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(host_thread_id) = state.hosts.get(&host_key).copied() {
        state.interrupted.insert(host_thread_id);
    }
}

#[test]
fn test_java_thread_hosts_toctou() {
    loom::model(|| {
        let state = Arc::new(RwLock::new(JavaThreadState::default()));

        let state1 = state.clone();
        let state2 = state.clone();

        // Setup state
        register_java_host_thread(&state, 1, 100);

        let t1 = thread::spawn(move || {
            unregister_java_host_thread(&state1, 1);
        });

        let t2 = thread::spawn(move || {
            // Simulate native_thread_interrupt
            interrupt_host_thread(&state2, 1);
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // The host key was removed, so it shouldn't be in interrupted either.
        assert!(
            state.read().unwrap().interrupted.is_empty(),
            "Thread leaked into interrupted map!"
        );
    });
}
