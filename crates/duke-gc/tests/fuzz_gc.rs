use duke_gc::Heap;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 10000, .. ProptestConfig::default() })]
    #[test]
    fn test_gc_heap_alloc_fuzz(
        class_name in ".*",
        size in 0..1024usize,
    ) {
        let mut heap = Heap::new();
        let _ = heap.allocate(class_name, size);
    }

    #[test]
    fn test_gc_fuzz_young_refs(
        refs in proptest::collection::vec(any::<u64>(), 0..100)
    ) {
        let mut heap = Heap::new();
        for _ in 0..10 {
            let _ = heap.allocate("java/lang/Object".to_string(), 1);
        }

        let roots: Vec<_> = refs.into_iter().map(|r| duke_runtime::Slot::Reference(Some(r))).collect();
        heap.minor_collect_prepare(&roots);
    }
}
