#[cfg(test)]
mod tests {

    use super::super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn fuzz_property_overrides(key in "\\PC*", value in "\\PC*") {
            let mut heap = duke_gc::Heap::new();
            let mut out = Vec::new();
            let mut control = NativeControl::default();

            let key_ref = heap.allocate_string(key.clone());
            let value_ref = heap.allocate_string(value.clone());

            let _ = native_system_set_property(
                &[Slot::Reference(Some(key_ref)), Slot::Reference(Some(value_ref))],
                &mut heap,
                &mut out,
                &mut control
            );

            let _ = native_system_get_property(
                &[Slot::Reference(Some(key_ref))],
                &mut heap,
                &mut out,
                &mut control
            );
        }
    }
}

#[cfg(all(test, feature = "loom"))]
mod loom_tests {
    #[test]
    fn loom_rwlock_concurrency() {
        loom::model(|| {
            let t1 = loom::thread::spawn(|| {
                crate::system_property_overrides()
                    .write()
                    .unwrap()
                    .insert("a".to_string(), "b".to_string());
            });
            let t2 = loom::thread::spawn(|| {
                let _ = crate::system_property_overrides().read().unwrap().get("a");
            });
            t1.join().unwrap();
            t2.join().unwrap();
        });
    }
}
