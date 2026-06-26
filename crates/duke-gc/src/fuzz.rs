#![allow(missing_docs)]

#[cfg(test)]
mod tests {
    use crate::Heap;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn does_not_crash_heap_allocations(
            size in 0usize..1000,
            string_len in 0usize..1000,
        ) {
            let mut heap = Heap::new();
            let _ = heap.allocate("java/lang/Object".to_string(), size);

            let s = "a".repeat(string_len);
            let _ = heap.allocate_string(s);
        }

        #[test]
        fn no_crash_oob_field_access(field_idx in 0usize..100) {
            use duke_runtime::Slot;
            use duke_runtime::Error;
            let mut heap = Heap::new();
            let obj_id = heap.allocate("java/lang/Object".to_string(), 10);
            let res = heap.write_field(obj_id, field_idx, Slot::Int(1));
            if field_idx >= 10 {
                assert!(matches!(res, Err(Error::FieldOutOfBounds { .. })));
            } else {
                assert!(res.is_ok());
            }
        }
    }
}
