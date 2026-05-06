#![allow(missing_docs)]
use loom::sync::{Arc, RwLock};
use loom::thread;
use std::collections::HashSet;

#[test]
fn havoc_interrupted_host_threads() {
    loom::model(|| {
        let interrupted = Arc::new(RwLock::new(HashSet::<usize>::new()));

        let interrupted1 = interrupted.clone();
        let t1 = thread::spawn(move || {
            let mut lock = interrupted1.write().unwrap();
            lock.insert(1);
        });

        let interrupted2 = interrupted.clone();
        let t2 = thread::spawn(move || {
            let lock = interrupted2.read().unwrap();
            let _ = lock.contains(&1);
        });

        let t3 = thread::spawn(move || {
            let mut lock = interrupted.write().unwrap();
            lock.remove(&1);
        });

        t1.join().unwrap();
        t2.join().unwrap();
        t3.join().unwrap();
    });
}
