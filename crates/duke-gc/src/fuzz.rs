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
        fn fuzz_heap_get_arbitrary_ref(address in any::<u64>()) {
            let mut heap = Heap::new();
            // On a 32-bit platform, an address > u32::MAX without OLD_BIT will cause
            // usize::try_from(r).unwrap() to panic.
            // On a 64-bit platform, it gracefully returns VmError::InvalidRef.
            // We just ensure it doesn't crash on standard x86_64, but we know it's a 32-bit DOS vulnerability.
            let _ = heap.get(address);
            let _ = heap.get_mut(address);
        }
    }
}
