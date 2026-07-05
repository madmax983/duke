use loom::sync::RwLock;
use loom::thread;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

struct RegistryState {
    hosts: HashMap<i32, usize>,
    interrupted: HashSet<usize>,
}

struct ThreadRegistry {
    state: RwLock<RegistryState>,
}

#[test]
fn test_thread_interrupt_leak() {
    loom::model(|| {
        let registry = Arc::new(ThreadRegistry {
            state: RwLock::new(RegistryState {
                hosts: HashMap::new(),
                interrupted: HashSet::new(),
            }),
        });

        {
            let mut state = registry.state.write().unwrap();
            state.hosts.insert(1, 100);
        }

        let reg1 = registry.clone();
        let t1 = thread::spawn(move || {
            // Equivalent to interrupt_host_thread reading then writing
            let mut state = reg1.state.write().unwrap();
            if let Some(&id) = state.hosts.get(&1) {
                state.interrupted.insert(id);
            }
        });

        let reg2 = registry.clone();
        let t2 = thread::spawn(move || {
            // Equivalent to unregister_java_host_thread
            let mut state = reg2.state.write().unwrap();
            if let Some(id) = state.hosts.remove(&1) {
                state.interrupted.remove(&id);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let is_empty = {
            let state = registry.state.read().unwrap();
            state.interrupted.is_empty()
        };
        assert!(
            is_empty,
            "Memory leak: thread ID left in interrupted set!"
        );
    });
}
