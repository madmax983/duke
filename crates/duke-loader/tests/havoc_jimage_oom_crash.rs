#![allow(missing_docs)]
#![allow(unused_imports)]
#![allow(clippy::collection_is_never_read)]
use duke_loader::jimage::JImageReader;
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
fn test_jimage_oom_crash() {
    let mut data = Vec::new();
    data.extend_from_slice(&0xCAFE_DADA_u32.to_le_bytes()); // magic
    data.extend_from_slice(&0x0001_0000_u32.to_le_bytes()); // version
    data.extend_from_slice(&0_u32.to_le_bytes());           // flags
    data.extend_from_slice(&1_u32.to_le_bytes());           // resource count
    data.extend_from_slice(&1_u32.to_le_bytes());           // table length
    data.extend_from_slice(&12_u32.to_le_bytes());          // locations size
    data.extend_from_slice(&14_u32.to_le_bytes());          // strings size

    data.extend_from_slice(&0_i32.to_le_bytes()); // redirect table
    data.extend_from_slice(&0_u32.to_le_bytes()); // offsets table

    // Location attributes
    data.push(0x08); // module name
    data.push(0x01); // len 1, value 1

    data.push(0x18); // uncompressed size
    data.push(0x05);
    data.push(0xFF); // LSB
    data.push(0xFF);
    data.push(0xFF);
    data.push(0x0F); // MSB -> 0x0FFFFFFF
    data.push(0x00);

    data.push(0x3B); // offset
    data.extend_from_slice(&[0x1F, 0xFF, 0xFF, 0xFF]);
    data.push(0x00);

    data.push(0x00); // EOF

    // Strings
    data.extend_from_slice(b"mod\0");
    data.extend_from_slice(b"Foo\0");
    data.extend_from_slice(&[0, 0, 0, 0, 0]);

    // Data (deflated)
    let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
    for _ in 0..1024 {
        std::io::Write::write_all(&mut encoder, &[0; 1024]).unwrap();
    }
    let compressed_data = encoder.finish().unwrap();
    data.extend_from_slice(&compressed_data[..20]);

    let path = std::env::temp_dir().join("havoc_jimage_oom_crash.jimage");
    std::fs::write(&path, &data).unwrap();

    ALLOCATED.store(0, Ordering::SeqCst);

    let reader = JImageReader::open(&path).unwrap();
    let _res = reader.read_resource("/mod/Foo");

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(
        allocated < 40_000_000,
        "Allocated {allocated} bytes! OOM triggered."
    );

    std::fs::remove_file(&path).unwrap();
}
