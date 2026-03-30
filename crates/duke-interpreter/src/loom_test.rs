#[cfg(test)]
mod tests {
    use loom::sync::{Arc, Mutex};
    use loom::thread;

    #[test]
    fn test_concurrent_threading_updates() {
        loom::model(|| {
            let buffer = Arc::new(Mutex::new(Vec::new()));

            let b1 = buffer.clone();
            let t1 = thread::spawn(move || {
                let mut guard = b1.lock().unwrap();
                guard.push(1);
            });

            let b2 = buffer.clone();
            let t2 = thread::spawn(move || {
                let mut guard = b2.lock().unwrap();
                guard.push(2);
            });

            t1.join().unwrap();
            t2.join().unwrap();

            let guard = buffer.lock().unwrap();
            assert_eq!(guard.len(), 2);
        });
    }
}
