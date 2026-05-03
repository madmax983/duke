#![allow(missing_docs)]
use loom::sync::Mutex;
use loom::thread;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

// In Loom tests, we must use `loom::thread::current().id()` instead of `std::thread::current().id()`
// to correctly integrate with Loom's deterministic thread scheduler.
// This test simulates the fix applied to PendingExceptionMessages.

#[test]
fn test_pending_exception_state_leak() {
    loom::model(|| {
        let messages = Arc::new(Mutex::new(HashMap::<
            (loom::thread::ThreadId, String),
            VecDeque<String>,
        >::new()));

        let m1 = messages.clone();
        let t1 = thread::spawn(move || {
            let class_name = "java/lang/IllegalArgumentException".to_string();
            let key = (loom::thread::current().id(), class_name);
            m1.lock()
                .unwrap()
                .entry(key.clone())
                .or_default()
                .push_back("thread1_error".to_string());
            m1.lock()
                .unwrap()
                .get_mut(&key)
                .unwrap()
                .pop_front()
                .unwrap()
        });

        let m2 = messages;
        let t2 = thread::spawn(move || {
            let class_name = "java/lang/IllegalArgumentException".to_string();
            let key = (loom::thread::current().id(), class_name);
            m2.lock()
                .unwrap()
                .entry(key.clone())
                .or_default()
                .push_back("thread2_error".to_string());
            m2.lock()
                .unwrap()
                .get_mut(&key)
                .unwrap()
                .pop_front()
                .unwrap()
        });

        let r1 = t1.join().unwrap();
        let r2 = t2.join().unwrap();

        assert_eq!(
            r1, "thread1_error",
            "Thread 1 received wrong exception message!"
        );
        assert_eq!(
            r2, "thread2_error",
            "Thread 2 received wrong exception message!"
        );
    });
}
