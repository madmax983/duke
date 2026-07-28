#![allow(missing_docs)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
use duke_loader::ZipReader;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

struct TrackingAllocator;

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
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

#[test]
fn havoc_zip_reader_read_entry_info_stored_no_oom() {
    let uncompressed_size: u64 = 300 * 1024 * 1024; // > 256MB

    // We create a valid zip file.
    let mut data = vec![0; 30];
    data[0..4].copy_from_slice(&0x0403_4b50_u32.to_le_bytes()); // LOCAL_SIGNATURE

    let mut cd_data = vec![];
    cd_data.extend_from_slice(&0x0201_4b50_u32.to_le_bytes()); // CD_SIGNATURE
    cd_data.extend_from_slice(&20_u16.to_le_bytes());
    cd_data.extend_from_slice(&20_u16.to_le_bytes());
    cd_data.extend_from_slice(&0_u16.to_le_bytes());
    cd_data.extend_from_slice(&0_u16.to_le_bytes()); // METHOD_STORED
    cd_data.extend_from_slice(&0_u16.to_le_bytes());
    cd_data.extend_from_slice(&0_u16.to_le_bytes());
    cd_data.extend_from_slice(&0_u32.to_le_bytes()); // crc
    cd_data.extend_from_slice(&((uncompressed_size % (u32::MAX as u64 + 1)) as u32).to_le_bytes()); // compressed size
    cd_data.extend_from_slice(&((uncompressed_size % (u32::MAX as u64 + 1)) as u32).to_le_bytes()); // uncompressed size
    cd_data.extend_from_slice(&8_u16.to_le_bytes()); // name_len
    cd_data.extend_from_slice(&0_u16.to_le_bytes());
    cd_data.extend_from_slice(&0_u16.to_le_bytes());
    cd_data.extend_from_slice(&0_u16.to_le_bytes());
    cd_data.extend_from_slice(&0_u16.to_le_bytes());
    cd_data.extend_from_slice(&0_u32.to_le_bytes());
    cd_data.extend_from_slice(&0_u32.to_le_bytes()); // local_offset
    cd_data.extend_from_slice(b"fuzz.txt");

    // To pass the "data extends past end of archive" check, we MUST have at least `30 + name_len + extra_len + compressed_size` bytes in `data`
    // However, allocating 300MB will trigger our tracker, which is what we want!
    data.resize(30 + 8 + uncompressed_size as usize, 0);

    let cd_offset = data.len() as u32;
    data.extend_from_slice(&cd_data);

    let cd_size = (data.len() as u32) - cd_offset;

    // EOCD
    data.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    data.extend_from_slice(&0_u16.to_le_bytes());
    data.extend_from_slice(&0_u16.to_le_bytes());
    data.extend_from_slice(&1_u16.to_le_bytes());
    data.extend_from_slice(&1_u16.to_le_bytes());
    data.extend_from_slice(&cd_size.to_le_bytes());
    data.extend_from_slice(&cd_offset.to_le_bytes());
    data.extend_from_slice(&0_u16.to_le_bytes());

    let reader = ZipReader::from_bytes(data).unwrap();

    // reset tracker
    ALLOCATED.store(0, Ordering::SeqCst);

    let res = reader.read_entry("fuzz.txt");

    let alloced = ALLOCATED.load(Ordering::SeqCst);

    // The issue is it allows > 256MB allocation. We want to reject it immediately!
    assert!(
        res.is_err(),
        "Vulnerability: Should have rejected the huge STORED entry"
    );
    assert!(
        alloced < 200 * 1024 * 1024,
        "Vulnerability: Allocated {alloced} bytes, but it should have failed early"
    );
}
