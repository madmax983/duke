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
#[allow(clippy::cast_possible_truncation, clippy::unreadable_literal)]
fn test_zip_stored_oom() {
    let mut data = Vec::new();

    // Create a fake Zip file with 1 large stored entry
    // Local file header
    data.extend_from_slice(&0x0403_4b50_u32.to_le_bytes()); // Local file header signature
    data.extend_from_slice(&10_u16.to_le_bytes()); // Version needed to extract
    data.extend_from_slice(&0_u16.to_le_bytes()); // General purpose bit flag
    data.extend_from_slice(&0_u16.to_le_bytes()); // Compression method (STORED)
    data.extend_from_slice(&0_u16.to_le_bytes()); // Last mod file time
    data.extend_from_slice(&0_u16.to_le_bytes()); // Last mod file date
    data.extend_from_slice(&0_u32.to_le_bytes()); // CRC-32 (0 for now)

    // We want the compressed size to be 256MB + 1 (0x10000001)
    let size: u32 = 256 * 1024 * 1024 + 1;
    data.extend_from_slice(&size.to_le_bytes()); // Compressed size
    data.extend_from_slice(&size.to_le_bytes()); // Uncompressed size

    let name = b"giant.txt";
    data.extend_from_slice(&(name.len() as u16).to_le_bytes()); // File name length
    data.extend_from_slice(&0_u16.to_le_bytes()); // Extra field length

    data.extend_from_slice(name); // File name

    let data_start = data.len() as u32;

    data.resize(data_start as usize + size as usize, 0);

    let cd_offset = data.len() as u32;

    let mut cd = Vec::new();
    cd.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
    cd.extend_from_slice(&10_u16.to_le_bytes());
    cd.extend_from_slice(&10_u16.to_le_bytes());
    cd.extend_from_slice(&0_u16.to_le_bytes());
    cd.extend_from_slice(&0_u16.to_le_bytes()); // STORED
    cd.extend_from_slice(&0_u16.to_le_bytes());
    cd.extend_from_slice(&0_u16.to_le_bytes());
    cd.extend_from_slice(&0_u32.to_le_bytes());
    cd.extend_from_slice(&size.to_le_bytes());
    cd.extend_from_slice(&size.to_le_bytes());
    cd.extend_from_slice(&(name.len() as u16).to_le_bytes());
    cd.extend_from_slice(&0_u16.to_le_bytes());
    cd.extend_from_slice(&0_u16.to_le_bytes());
    cd.extend_from_slice(&0_u16.to_le_bytes());
    cd.extend_from_slice(&0_u16.to_le_bytes());
    cd.extend_from_slice(&0_u32.to_le_bytes());
    cd.extend_from_slice(&0_u32.to_le_bytes());
    cd.extend_from_slice(name);

    let cd_size = cd.len() as u32;
    data.extend_from_slice(&cd);

    data.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    data.extend_from_slice(&0_u16.to_le_bytes());
    data.extend_from_slice(&0_u16.to_le_bytes());
    data.extend_from_slice(&1_u16.to_le_bytes());
    data.extend_from_slice(&1_u16.to_le_bytes());
    data.extend_from_slice(&cd_size.to_le_bytes());
    data.extend_from_slice(&cd_offset.to_le_bytes());
    data.extend_from_slice(&0_u16.to_le_bytes());

    let reader = ZipReader::from_bytes(data).unwrap();

    ALLOCATED.store(0, Ordering::SeqCst);

    let _res = reader.read_entry("giant.txt");

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(
        allocated < 256 * 1024 * 1024,
        "Allocated {allocated} bytes! OOM triggered."
    );
}
