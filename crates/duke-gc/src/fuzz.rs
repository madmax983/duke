use proptest::prelude::*;

proptest! {
    #[test]
    fn does_not_crash_heap_allocations(
        class_names in proptest::collection::vec(".*", 0..10),
        field_counts in proptest::collection::vec(0..10usize, 0..10),
    ) {
        let mut heap = crate::Heap::new();
        for (name, count) in class_names.into_iter().zip(field_counts) {
            heap.allocate(name, count);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn does_not_crash_on_huge_references() {
        let mut heap = crate::Heap::new();

        let r = !crate::OLD_BIT;
        assert!(matches!(heap.get(r), Err(duke_runtime::VmError::InvalidRef { .. })));
        assert!(matches!(heap.get_mut(r), Err(duke_runtime::VmError::InvalidRef { .. })));

        let old_r = u64::MAX | crate::OLD_BIT;
        assert!(matches!(heap.get(old_r), Err(duke_runtime::VmError::InvalidRef { .. })));
        assert!(matches!(heap.get_mut(old_r), Err(duke_runtime::VmError::InvalidRef { .. })));

        // Test minor_collect_prepare with a huge ref
        let roots = vec![duke_runtime::Slot::Reference(Some(r))];
        heap.minor_collect_prepare(&roots); // Should ignore gracefully since get() handles large indices.
    }
}
