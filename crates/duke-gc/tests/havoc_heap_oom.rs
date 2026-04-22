#![allow(missing_docs)]
use duke_gc::Heap;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

#[test]
#[should_panic(expected = "Allocation exceeded maximum allowed field count")]
fn test_heap_allocate_oom() {
    ALLOCATED.store(0, Ordering::SeqCst);
    let mut heap = Heap::new();
    let _ = heap.allocate("Ljava/lang/Object;".to_string(), 2_000_000_000);
}
