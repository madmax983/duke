#![allow(missing_docs)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
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
fn test_zip_stored_oom() {
    let max_size = 1024 * 1024 * 256;

    // Create a dummy zip file that is large
    // actually, it's easier to manually construct the zip fields
    // to have compressed_size > max_size.
    let mut zip = Vec::new();

    // We just use a basic structure.
    let name = "huge.txt";
    let crc = 0_u32;
    let size = (max_size + 10) as u32;

    // Local header
    zip.extend_from_slice(&0x0403_4b50_u32.to_le_bytes()); // LOCAL_SIGNATURE
    zip.extend_from_slice(&20_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes()); // METHOD_STORED
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&crc.to_le_bytes());
    zip.extend_from_slice(&size.to_le_bytes());
    zip.extend_from_slice(&size.to_le_bytes());
    zip.extend_from_slice(&(name.len() as u16).to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(name.as_bytes());

    // Now we append dummy data up to size
    zip.resize(zip.len() + size as usize, 0);

    let cd_offset = zip.len() as u32;

    zip.extend_from_slice(&0x0201_4b50_u32.to_le_bytes()); // CD_SIGNATURE
    zip.extend_from_slice(&20_u16.to_le_bytes());
    zip.extend_from_slice(&20_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes()); // METHOD_STORED
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&crc.to_le_bytes());
    zip.extend_from_slice(&size.to_le_bytes());
    zip.extend_from_slice(&size.to_le_bytes());
    zip.extend_from_slice(&(name.len() as u16).to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u32.to_le_bytes());
    zip.extend_from_slice(&0_u32.to_le_bytes()); // local offset
    zip.extend_from_slice(name.as_bytes());

    let cd_size = zip.len() as u32 - cd_offset;

    // EOCD
    zip.extend_from_slice(&0x0605_4b50_u32.to_le_bytes()); // EOCD_SIGNATURE
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&1_u16.to_le_bytes());
    zip.extend_from_slice(&1_u16.to_le_bytes());
    zip.extend_from_slice(&cd_size.to_le_bytes());
    zip.extend_from_slice(&cd_offset.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());

    let reader = ZipReader::from_bytes(zip).unwrap();

    ALLOCATED.store(0, Ordering::SeqCst);

    let _ = reader.read_entry("huge.txt");

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(
        allocated < 256 * 1024 * 1024,
        "Allocated {allocated} bytes! OOM triggered."
    );
}
