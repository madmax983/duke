#![allow(missing_docs)]
use loom::sync::RwLock;
use loom::thread;
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn test_zip_files_deadlock() {
    loom::model(|| {
        let zip_files = Arc::new(RwLock::new(HashMap::<i32, i32>::new()));

        let z1 = zip_files.clone();
        let t1 = thread::spawn(move || {
            let mut map = z1.write().unwrap();
            map.insert(1, 100);
        });

        let z2 = zip_files;
        let t2 = thread::spawn(move || {
            let map = z2.read().unwrap();
            // If another thread panics while holding a write lock, the rwlock might become poisoned.
            // But we can simulate a reader thread panicking while holding the read lock.
            // However, Loom tests panic naturally if a real deadlock or data race occurs.
            let _ = map.get(&1);
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}
