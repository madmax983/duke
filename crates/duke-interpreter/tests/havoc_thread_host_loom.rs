#![allow(missing_docs)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[test]
#[allow(clippy::significant_drop_tightening)]
fn test_thread_host_unregister_race_fix() {
    loom::model(|| {
        let hosts = Arc::new(RwLock::new(HashMap::<i32, i32>::new()));
        let interrupted = Arc::new(RwLock::new(HashSet::<i32>::new()));

        let host_key = 1;
        let thread_id = 100;

        hosts.write().unwrap().insert(host_key, thread_id);

        let h1 = hosts.clone();
        let i1 = interrupted.clone();
        let t1 = thread::spawn(move || {
            // Unregister thread
            let mut hosts_lock = h1.write().unwrap();
            let removed = hosts_lock.remove(&host_key);
            if let Some(tid) = removed {
                i1.write().unwrap().remove(&tid);
            }
        });

        let h2 = hosts;
        let i2 = interrupted.clone();
        let t2 = thread::spawn(move || {
            // Interrupt thread
            let hosts_lock = h2.read().unwrap();
            let tid = hosts_lock.get(&host_key).copied();
            if let Some(tid) = tid {
                i2.write().unwrap().insert(tid);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        assert!(
            interrupted.read().unwrap().is_empty(),
            "Memory leak: interrupted host thread leaked!"
        );
    });
}
