#![allow(missing_docs)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Default)]
struct ThreadRegistry {
    hosts: HashMap<i32, usize>,
    interrupted: HashSet<usize>,
}

#[test]
fn test_thread_hosts_race_condition() {
    loom::model(|| {
        let registry = Arc::new(RwLock::new(ThreadRegistry::default()));

        registry.write().unwrap().hosts.insert(1, 100);

        let r1 = registry.clone();
        let t1 = thread::spawn(move || {
            let mut reg = r1.write().unwrap();
            let removed = reg.hosts.remove(&1);
            if let Some(id) = removed {
                reg.interrupted.remove(&id);
            }
        });

        let r2 = registry.clone();
        let t2 = thread::spawn(move || {
            let mut reg = r2.write().unwrap();
            if let Some(id) = reg.hosts.get(&1).copied() {
                reg.interrupted.insert(id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let final_registry = registry.read().unwrap();
        assert!(
            final_registry.interrupted.is_empty(),
            "Race condition! Thread is unregistered but still interrupted: {:?}",
            final_registry.interrupted
        );
    });
}
