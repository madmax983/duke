#![allow(missing_docs)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[test]
fn test_unregister_race_toctou() {
    loom::model(|| {
        let hosts = Arc::new(RwLock::new(HashMap::<i32, loom::thread::ThreadId>::new()));
        let interrupted = Arc::new(RwLock::new(HashSet::<loom::thread::ThreadId>::new()));

        let h_init = hosts.clone();
        let tid1 = loom::thread::current().id();
        h_init.write().unwrap().insert(1, tid1);

        let h1 = hosts.clone();
        let i1 = interrupted.clone();

        let t1 = thread::spawn(move || {
            let removed_host_thread = h1.write().unwrap().remove(&1);
            if let Some(host_thread_id) = removed_host_thread {
                i1.write().unwrap().remove(&host_thread_id);
            }
        });

        let h2 = hosts;
        let i2 = interrupted.clone();
        let t2 = thread::spawn(move || {
            let h_guard = h2.read().unwrap();
            let host_thread_id = h_guard.get(&1).copied();

            if let Some(id) = host_thread_id {
                let mut i_guard = i2.write().unwrap();
                if h_guard.values().any(|&v| v == id) {
                    i_guard.insert(id);
                }
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        assert!(
            !interrupted.read().unwrap().contains(&tid1),
            "Memory leak in interrupted_host_threads due to TOCTOU race!"
        );
    });
}
