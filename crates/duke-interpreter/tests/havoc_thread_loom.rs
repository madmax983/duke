#![allow(missing_docs)]
use loom::sync::{Arc, RwLock};
use loom::thread;
use std::collections::{HashMap, HashSet};

#[test]
fn test_thread_registration_deadlock() {
    loom::model(|| {
        let hosts = Arc::new(RwLock::new(HashMap::<i32, loom::thread::ThreadId>::new()));
        let interrupted = Arc::new(RwLock::new(HashSet::<loom::thread::ThreadId>::new()));

        let h1 = hosts.clone();
        let i1 = interrupted.clone();
        let t1 = thread::spawn(move || {
            let host_key = 1;
            let removed_host_thread = h1.write().unwrap().remove(&host_key);
            if let Some(host_thread_id) = removed_host_thread {
                i1.write().unwrap().remove(&host_thread_id);
            }
        });

        let h2 = hosts;
        let i2 = interrupted;
        let t2 = thread::spawn(move || {
            let host_key = 1;
            let host_thread_id = h2.read().unwrap().get(&host_key).copied();
            if let Some(id) = host_thread_id {
                i2.write().unwrap().insert(id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}
