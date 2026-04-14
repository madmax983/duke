#![allow(missing_docs)]
use duke_classfile::parse;
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
fn test_oom_cp() {
    let mut data = vec![0xca, 0xfe, 0xba, 0xbe, 0x00, 0x00, 0x00, 0x41]; // magic and version
    // constant pool count: 0xffff
    data.push(0xff);
    data.push(0xff);

    ALLOCATED.store(0, Ordering::SeqCst);
    let _ = parse(&data);

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(allocated < 10_000_000, "Allocated {allocated} bytes");
}

#[test]
fn test_oom_methods() {
    let mut data = vec![0xca, 0xfe, 0xba, 0xbe, 0x00, 0x00, 0x00, 0x41]; // magic and version
    // constant pool count: 1
    data.push(0x00);
    data.push(0x01);

    // access flags
    data.push(0x00);
    data.push(0x00);
    // this class
    data.push(0x00);
    data.push(0x00);
    // super class
    data.push(0x00);
    data.push(0x00);

    // interfaces count
    data.push(0x00);
    data.push(0x00);

    // fields count
    data.push(0x00);
    data.push(0x00);

    // methods count: 0xffff
    data.push(0xff);
    data.push(0xff);

    ALLOCATED.store(0, Ordering::SeqCst);
    let _ = parse(&data);

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(allocated < 10_000_000, "Allocated {allocated} bytes");
}

#[test]
fn test_oom_interfaces() {
    let mut data = vec![0xca, 0xfe, 0xba, 0xbe, 0x00, 0x00, 0x00, 0x41]; // magic and version
    // constant pool count: 1
    data.push(0x00);
    data.push(0x01);

    // access flags
    data.push(0x00);
    data.push(0x00);
    // this class
    data.push(0x00);
    data.push(0x00);
    // super class
    data.push(0x00);
    data.push(0x00);

    // interfaces count
    data.push(0xff);
    data.push(0xff);

    ALLOCATED.store(0, Ordering::SeqCst);
    let _ = parse(&data);

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(allocated < 10_000_000, "Allocated {allocated} bytes");
}
