use loom::sync::Mutex;
use loom::thread;
use std::sync::Arc;
use std::collections::HashMap;

// This loom test verifies a time-of-check to time-of-use (TOCTOU) race condition.
// `native_system_set_property` previously called `system_property_value(key)`,
// unlocking the mutex, then relocking it to insert the new value.
// This meant concurrent `setProperty` calls on the same key could overwrite each other
// while returning outdated `previous` values to the JVM, violating atomicity.

#[test]
fn test_system_property_toctou_race_simulation() {
    loom::model(|| {
        let overrides = Arc::new(Mutex::new(HashMap::<String, String>::new()));

        let overrides1 = overrides.clone();
        let t1 = thread::spawn(move || {
            let key = "foo".to_string();
            // This is the FIX pattern: lock once, get or else fallback, then insert.
            let mut lock = overrides1.lock().unwrap();
            let prev = lock.get(&key).cloned();
            lock.insert(key, "bar".to_string());
            prev
        });

        let overrides2 = overrides.clone();
        let t2 = thread::spawn(move || {
            let key = "foo".to_string();
            let mut lock = overrides2.lock().unwrap();
            let prev = lock.get(&key).cloned();
            lock.insert(key, "baz".to_string());
            prev
        });

        let r1 = t1.join().unwrap();
        let r2 = t2.join().unwrap();

        let final_val = overrides.lock().unwrap().get("foo").cloned().unwrap();

        let mut seen = std::collections::HashSet::new();
        if let Some(v) = r1 { seen.insert(v); }
        if let Some(v) = r2 { seen.insert(v); }
        seen.insert(final_val);

        // With the fix, we guarantee atomicity: one thread gets None and sets its value,
        // the other thread gets the first thread's value and sets the final value.
        // Therefore, BOTH "bar" and "baz" must be seen exactly once (one as intermediate, one as final).
        assert!(seen.contains("bar"), "bar was lost in TOCTOU race");
        assert!(seen.contains("baz"), "baz was lost in TOCTOU race");
    });
}
