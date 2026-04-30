#![allow(missing_docs)]
use duke_loader::ZipReader;
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
fn test_zip_cd_oom() {
    let mut data = Vec::new();

    // Create a central directory that advertises a huge number of entries
    // EOCD record
    data.extend_from_slice(&0x0605_4b50_u32.to_le_bytes()); // EOCD signature
    data.extend_from_slice(&0_u16.to_le_bytes()); // number of this disk
    data.extend_from_slice(&0_u16.to_le_bytes()); // disk where central directory starts
    data.extend_from_slice(&0xffff_u16.to_le_bytes()); // number of central directory records on this disk (HUGE)
    data.extend_from_slice(&0xffff_u16.to_le_bytes()); // total number of central directory records (HUGE)
    data.extend_from_slice(&0_u32.to_le_bytes()); // size of central directory
    data.extend_from_slice(&0_u32.to_le_bytes()); // offset of start of central directory
    data.extend_from_slice(&0_u16.to_le_bytes()); // ZIP file comment length

    ALLOCATED.store(0, Ordering::SeqCst);
    let _ = ZipReader::from_bytes(data);
    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(allocated < 10_000_000, "Allocated {allocated} bytes");
}
