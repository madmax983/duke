use loom::sync::RwLock;
use std::collections::{HashMap, HashSet};

struct ThreadHosts {
    hosts: HashMap<i32, loom::thread::ThreadId>,
    interrupted: HashSet<loom::thread::ThreadId>,
}

fn thread_hosts() -> &'static RwLock<ThreadHosts> {
    loom::lazy_static! {
        static ref HOSTS: RwLock<ThreadHosts> = RwLock::new(ThreadHosts {
            hosts: HashMap::new(),
            interrupted: HashSet::new(),
        });
    }
    &HOSTS
}

fn register_java_host_thread(host_key: i32, host_thread_id: loom::thread::ThreadId) {
    let mut state = thread_hosts().write().unwrap();
    state.hosts.insert(host_key, host_thread_id);
}

fn unregister_java_host_thread(host_key: i32) {
    let mut state = thread_hosts().write().unwrap();
    let removed_host_thread = state.hosts.remove(&host_key);
    if let Some(host_thread_id) = removed_host_thread {
        state.interrupted.remove(&host_thread_id);
    }
}

fn interrupt_java_host_thread(host_key: i32) {
    let mut state = thread_hosts().write().unwrap();
    if let Some(host_thread_id) = state.hosts.get(&host_key).copied() {
        state.interrupted.insert(host_thread_id);
    }
}

#[test]
fn test_thread_unregister_race() {
    loom::model(|| {
        {
            let mut state = thread_hosts().write().unwrap();
            state.hosts.clear();
            state.interrupted.clear();
        }

        let host_key = 1;
        let main_thread = loom::thread::current().id();
        register_java_host_thread(host_key, main_thread);

        let t1 = loom::thread::spawn(move || {
            // The race condition is fixed by holding the lock for both checking
            // the registered thread ID and inserting it into the interrupted set.
            interrupt_java_host_thread(host_key);
        });

        let t2 = loom::thread::spawn(move || {
            unregister_java_host_thread(host_key);
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let is_empty = thread_hosts().read().unwrap().interrupted.is_empty();
        assert!(
            is_empty,
            "Leak detected: Interrupted set contains un-registered thread"
        );
    });
}
