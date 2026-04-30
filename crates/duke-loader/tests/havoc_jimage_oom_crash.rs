#![allow(missing_docs)]
use duke_loader::JImageReader;
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
fn havoc_crash_build_index_alloc() {
    let mut data = Vec::new();
    data.extend_from_slice(&0xCAFE_DADA_u32.to_le_bytes()); // magic
    data.extend_from_slice(&0x0001_0000_u32.to_le_bytes()); // version
    data.extend_from_slice(&0_u32.to_le_bytes());           // flags
    data.extend_from_slice(&0x00FF_FFFF_u32.to_le_bytes());    // resource count (HUGE)
    data.extend_from_slice(&1_u32.to_le_bytes());           // table length
    data.extend_from_slice(&12_u32.to_le_bytes());          // loc attrs size
    data.extend_from_slice(&14_u32.to_le_bytes());          // string table size

    // Provide a small file and see if we allocate table_length * 4 strings
    let path = std::env::temp_dir().join("havoc_jimage_crash.jimage");
    std::fs::write(&path, &data).unwrap();

    ALLOCATED.store(0, Ordering::SeqCst);
    let _ = JImageReader::open(&path);
    let allocated = ALLOCATED.load(Ordering::SeqCst);

    std::fs::remove_file(&path).unwrap();

    assert!(allocated < 50_000_000, "Allocated {allocated} bytes (OOM)");
}
