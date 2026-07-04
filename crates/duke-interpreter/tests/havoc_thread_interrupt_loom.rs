#![allow(missing_docs)]
#![allow(clippy::significant_drop_tightening)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

// We create a mock that has the EXACT same layout as the code in native.rs
fn java_thread_hosts(
    once: &OnceLock<RwLock<HashMap<i32, loom::thread::ThreadId>>>,
) -> &RwLock<HashMap<i32, loom::thread::ThreadId>> {
    once.get_or_init(|| RwLock::new(HashMap::new()))
}

fn interrupted_host_threads(
    once: &OnceLock<RwLock<HashSet<loom::thread::ThreadId>>>,
) -> &RwLock<HashSet<loom::thread::ThreadId>> {
    once.get_or_init(|| RwLock::new(HashSet::new()))
}

#[test]
fn test_thread_interrupt_toctou_leak() {
    loom::model(|| {
        let hosts_once = Arc::new(OnceLock::new());
        let interrupted_once = Arc::new(OnceLock::new());

        // Setup: Thread is registered
        let target_thread_id = loom::thread::current().id();
        java_thread_hosts(&hosts_once)
            .write()
            .unwrap()
            .insert(1, target_thread_id);

        let ho1 = hosts_once.clone();
        let io1 = interrupted_once.clone();
        let t1 = thread::spawn(move || {
            // FIX for `native_thread_interrupt`: keep `hosts_guard` open!
            let hosts_guard = java_thread_hosts(&ho1).read().unwrap();
            let target_thread_id = hosts_guard.get(&1).copied();

            if let Some(id) = target_thread_id {
                interrupted_host_threads(&io1).write().unwrap().insert(id);
            }
            drop(hosts_guard);
        });

        // let ho2 = hosts_once.clone(); removed redundant clone
        let io2 = interrupted_once.clone();
        let t2 = thread::spawn(move || {
            // FIX for `unregister_java_host_thread`: keep `hosts_guard` open!
            let mut hosts_guard = java_thread_hosts(&hosts_once).write().unwrap();
            let removed_host_thread = hosts_guard.remove(&1);
            if let Some(id) = removed_host_thread {
                interrupted_host_threads(&io2).write().unwrap().remove(&id);
            }
            drop(hosts_guard);
        });

        t1.join().unwrap();
        t2.join().unwrap();

        assert!(
            !interrupted_host_threads(&interrupted_once)
                .read()
                .unwrap()
                .contains(&target_thread_id),
            "Leaked interrupted thread ID due to TOCTOU!"
        );
    });
}
