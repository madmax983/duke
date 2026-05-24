#![allow(missing_docs)]
#![allow(clippy::significant_drop_tightening)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[test]
fn test_thread_hosts_read_write_deadlock() {
    loom::model(|| {
        let hosts = Arc::new(RwLock::new(HashMap::<i32, loom::thread::ThreadId>::new()));
        let interrupted = Arc::new(RwLock::new(HashSet::<loom::thread::ThreadId>::new()));

        let h1 = hosts.clone();
        let i1 = interrupted.clone();
        let t1 = thread::spawn(move || {
            let mut h_lock = h1.write().unwrap();
            let removed = h_lock.remove(&1);
            if let Some(id) = removed {
                let mut i_lock = i1.write().unwrap();
                i_lock.remove(&id);
            }
        });

        let h2 = hosts;
        let i2 = interrupted;
        let t2 = thread::spawn(move || {
            let current_id = loom::thread::current().id();
            let h_lock = h2.read().unwrap();

            // Acquire the interrupted lock while holding the h_lock
            let mut i_lock = i2.write().unwrap();
            i_lock.insert(current_id);
            let _ = h_lock
                .iter()
                .find_map(|(k, v)| if *v == current_id { Some(*k) } else { None });
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}
