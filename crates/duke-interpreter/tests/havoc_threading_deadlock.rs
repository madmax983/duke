use std::sync::{Arc, Mutex};
use std::thread;
use std::collections::HashMap;

struct CompletionRuntime {
    handles: HashMap<i32, thread::JoinHandle<()>>,
}

#[test]
fn havoc_test_deadlock() {
    let runtime = Arc::new(Mutex::new(CompletionRuntime { handles: HashMap::new() }));

    let handle1 = thread::spawn(move || {
        thread::sleep(std::time::Duration::from_millis(100));
    });
    runtime.lock().unwrap().handles.insert(1, handle1);

    let rt2 = runtime.clone();
    let handle2 = thread::spawn(move || {
        loop {
            let handle = rt2.lock().unwrap().handles.remove(&1);
            if let Some(h) = handle {
                let _ = h.join();
                break;
            }
            thread::yield_now();
        }
    });
    runtime.lock().unwrap().handles.insert(2, handle2);

    // Simulate wait_for_all_java_threads
    loop {
        let handles = {
            let mut rt = runtime.lock().unwrap();
            if rt.handles.is_empty() {
                break;
            }
            rt.handles.drain().map(|(_, h)| h).collect::<Vec<_>>()
        };

        for h in handles {
            let _ = h.join();
        }
    }
}
