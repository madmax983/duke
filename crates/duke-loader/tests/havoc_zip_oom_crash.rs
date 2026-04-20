#![allow(missing_docs)]
#![allow(clippy::cast_possible_truncation)]
use duke_loader::zip::ZipReader;
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
fn test_zip_oom_crash() {
    let mut zip = Vec::new();
    let local_offset = zip.len() as u32;
    zip.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
    zip.extend_from_slice(&20_u16.to_le_bytes()); // version
    zip.extend_from_slice(&0_u16.to_le_bytes());  // flags
    zip.extend_from_slice(&8_u16.to_le_bytes());  // method: deflated
    zip.extend_from_slice(&0_u16.to_le_bytes());  // time
    zip.extend_from_slice(&0_u16.to_le_bytes());  // date
    zip.extend_from_slice(&0_u32.to_le_bytes());  // crc32

    let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
    for _ in 0..1024 {
        std::io::Write::write_all(&mut encoder, &[0; 1024]).unwrap();
    }
    let compressed_data = encoder.finish().unwrap();

    zip.extend_from_slice(&(u32::try_from(compressed_data.len()).unwrap()).to_le_bytes()); // compressed size
    zip.extend_from_slice(&0x0FFF_FFFF_u32.to_le_bytes()); // uncompressed size (~268MB)
    zip.extend_from_slice(&4_u16.to_le_bytes()); // name len
    zip.extend_from_slice(&0_u16.to_le_bytes()); // extra len
    zip.extend_from_slice(b"boom"); // name
    zip.extend_from_slice(&compressed_data);

    let cd_offset = u32::try_from(zip.len()).unwrap();
    zip.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
    zip.extend_from_slice(&20_u16.to_le_bytes());
    zip.extend_from_slice(&20_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&8_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u32.to_le_bytes());
    zip.extend_from_slice(&(u32::try_from(compressed_data.len()).unwrap()).to_le_bytes());
    zip.extend_from_slice(&0x0FFF_FFFF_u32.to_le_bytes()); // uncompressed size
    zip.extend_from_slice(&4_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u32.to_le_bytes());
    zip.extend_from_slice(&local_offset.to_le_bytes());
    zip.extend_from_slice(b"boom");

    let cd_size = u32::try_from(zip.len()).unwrap() - cd_offset;

    zip.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());
    zip.extend_from_slice(&1_u16.to_le_bytes());
    zip.extend_from_slice(&1_u16.to_le_bytes());
    zip.extend_from_slice(&cd_size.to_le_bytes());
    zip.extend_from_slice(&cd_offset.to_le_bytes());
    zip.extend_from_slice(&0_u16.to_le_bytes());

    ALLOCATED.store(0, Ordering::SeqCst);
    let reader = ZipReader::from_bytes(zip).unwrap();
    let _res = reader.read_entry("boom");

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(
        allocated < 40_000_000,
        "Allocated {allocated} bytes! OOM triggered."
    );
}
