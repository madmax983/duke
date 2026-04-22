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
fn test_frame_oom() {
    ALLOCATED.store(0, Ordering::SeqCst);
    let _ = Frame::new(usize::MAX, usize::MAX, vec![]);
    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(allocated < 10_000_000, "Allocated {allocated} bytes");
}
