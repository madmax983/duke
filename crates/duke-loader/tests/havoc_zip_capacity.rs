#![allow(missing_docs)]
//! Tests for Havoc ZIP OOM issues
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
fn havoc_test_oom() {
    let bad_data = vec![
        // EOCD signature
        0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0xff, 0xff, // 65535 entries
        0xff, 0xff, 0, 0, 0, 0, // cd size
        0, 0, 0, 0, // offset
        0, 0,
    ];
    ALLOCATED.store(0, Ordering::SeqCst);
    let res = duke_loader::ZipReader::from_bytes(bad_data);
    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(res.is_err());
    assert!(allocated < 10_000_000, "Allocated {allocated} bytes");
}
