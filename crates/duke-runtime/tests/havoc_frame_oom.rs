#![allow(missing_docs)]
use duke_runtime::Frame;
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
fn test_frame_capacity_overflow() {
    ALLOCATED.store(0, Ordering::SeqCst);

    // Test large max_stack. We expect it to gracefully return Err rather than panic in Vec::with_capacity
    let result1 = Frame::new(usize::MAX / 16, 5, vec![]);
    assert!(result1.is_err());

    // Test large max_locals. We expect it to gracefully return Err rather than panic in Vec::resize
    let result2 = Frame::new(5, usize::MAX / 16, vec![]);
    assert!(result2.is_err());

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(
        allocated < 10_000_000,
        "Allocated {allocated} bytes! OOM triggered."
    );
}
