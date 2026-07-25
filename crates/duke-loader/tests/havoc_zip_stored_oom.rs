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
fn test_havoc_zip_stored_oom_simulation() {
    let mut zip = Vec::new();
    let name = "huge.txt";
    let name_bytes = name.as_bytes();

    zip.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]);
    zip.extend_from_slice(&[20, 0]); // version
    zip.extend_from_slice(&[0, 0]); // flags
    zip.extend_from_slice(&[0, 0]); // compression = STORED
    zip.extend_from_slice(&[0, 0, 0, 0]); // time
    zip.extend_from_slice(&[0, 0, 0, 0]); // crc32

    let size = (1024 * 1024 * 257u32).to_le_bytes(); // 257 MB

    zip.extend_from_slice(&size); // compressed size
    zip.extend_from_slice(&size); // uncompressed size

    let name_len = (name_bytes.len() as u16).to_le_bytes();
    zip.extend_from_slice(&name_len); // name len
    zip.extend_from_slice(&[0, 0]); // extra len

    zip.extend_from_slice(name_bytes);

    zip.resize(zip.len() + (1024 * 1024 * 257), 0);

    let offset = zip.len();

    zip.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
    zip.extend_from_slice(&[20, 0]); // version made by
    zip.extend_from_slice(&[20, 0]); // version needed
    zip.extend_from_slice(&[0, 0]); // flags
    zip.extend_from_slice(&[0, 0]); // compression = STORED
    zip.extend_from_slice(&[0, 0, 0, 0]); // time
    zip.extend_from_slice(&[0, 0, 0, 0]); // crc32
    zip.extend_from_slice(&size); // compressed size
    zip.extend_from_slice(&size); // uncompressed size
    zip.extend_from_slice(&name_len); // name len
    zip.extend_from_slice(&[0, 0]); // extra len
    zip.extend_from_slice(&[0, 0]); // comment len
    zip.extend_from_slice(&[0, 0]); // disk
    zip.extend_from_slice(&[0, 0]); // attr
    zip.extend_from_slice(&[0, 0, 0, 0]); // ext attr
    zip.extend_from_slice(&[0, 0, 0, 0]); // offset
    zip.extend_from_slice(name_bytes);

    let cd_size = zip.len() - offset;

    zip.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
    zip.extend_from_slice(&[0, 0, 0, 0]);
    zip.extend_from_slice(&[1, 0, 1, 0]); // 1 entry
    zip.extend_from_slice(&(cd_size as u32).to_le_bytes()); // cd size
    zip.extend_from_slice(&(offset as u32).to_le_bytes()); // cd offset
    zip.extend_from_slice(&[0, 0]); // comment len

    let reader = ZipReader::from_bytes(zip).unwrap();
    ALLOCATED.store(0, Ordering::SeqCst);

    let res = reader.read_entry("huge.txt");
    let allocated = ALLOCATED.load(Ordering::SeqCst);

    assert!(res.is_err(), "Expected an error (limit exceeded), but it allocated {} bytes", allocated);
    assert!(allocated < 1024 * 1024 * 256, "Allocated {} bytes, bypassing OOM limit!", allocated);
}
