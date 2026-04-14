#![allow(missing_docs)]
use duke_bytecode::decode;
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
fn test_oom_lookupswitch() {
    let mut data = vec![0xab]; // lookupswitch
    data.push(0x00);
    data.push(0x00);
    data.push(0x00); // padding

    // default
    data.push(0x00);
    data.push(0x00);
    data.push(0x00);
    data.push(0x00);

    // npairs: 0x3fffffff
    data.push(0x3f);
    data.push(0xff);
    data.push(0xff);
    data.push(0xff);

    ALLOCATED.store(0, Ordering::SeqCst);
    let _ = decode(&data);

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(allocated < 10_000_000, "Allocated {allocated} bytes");
}

#[test]
fn test_oom_tableswitch() {
    let mut data = vec![0xaa]; // tableswitch
    data.push(0x00);
    data.push(0x00);
    data.push(0x00); // padding

    // default
    data.push(0x00);
    data.push(0x00);
    data.push(0x00);
    data.push(0x00);

    // low: 0
    data.push(0x00);
    data.push(0x00);
    data.push(0x00);
    data.push(0x00);

    // high: 0x3fffffff
    data.push(0x3f);
    data.push(0xff);
    data.push(0xff);
    data.push(0xff);

    ALLOCATED.store(0, Ordering::SeqCst);
    let _ = decode(&data);

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(allocated < 10_000_000, "Allocated {allocated} bytes");
}
