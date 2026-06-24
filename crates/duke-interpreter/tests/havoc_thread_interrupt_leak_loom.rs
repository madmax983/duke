#![allow(missing_docs)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

struct ThreadRegistry {
    hosts: HashMap<i32, loom::thread::ThreadId>,
    interrupted: HashSet<loom::thread::ThreadId>,
}

#[test]
fn test_thread_interrupt_state_leak() {
    loom::model(|| {
        let registry = Arc::new(RwLock::new(ThreadRegistry {
            hosts: HashMap::new(),
            interrupted: HashSet::new(),
        }));

        let main_tid = loom::thread::current().id();
        registry.write().unwrap().hosts.insert(1, main_tid);

        let r1 = registry.clone();
        let t1 = thread::spawn(move || {
            // unregister
            let mut reg = r1.write().unwrap();
            let removed = reg.hosts.remove(&1);
            if let Some(tid) = removed {
                reg.interrupted.remove(&tid);
            }
        });

        let r2 = registry.clone();
        let t2 = thread::spawn(move || {
            // native_thread_interrupt
            let mut reg = r2.write().unwrap();
            let tid = reg.hosts.get(&1).copied();
            if let Some(tid) = tid {
                reg.interrupted.insert(tid);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let is_leaked = registry.read().unwrap().interrupted.contains(&main_tid);
        assert!(!is_leaked, "Thread state leak detected!");
    });
}
