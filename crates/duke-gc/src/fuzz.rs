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
    }
}
