#![allow(missing_docs)]
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
fn test_jimage_oom() {
    let mut data = Vec::new();
    data.extend_from_slice(&0xCAFE_DADA_u32.to_le_bytes());
    data.extend_from_slice(&0x0001_0000_u32.to_le_bytes());
    data.extend_from_slice(&0_u32.to_le_bytes());
    data.extend_from_slice(&1_u32.to_le_bytes());
    data.extend_from_slice(&1_u32.to_le_bytes());
    data.extend_from_slice(&12_u32.to_le_bytes());
    data.extend_from_slice(&14_u32.to_le_bytes());

    data.extend_from_slice(&0_i32.to_le_bytes());
    data.extend_from_slice(&0_u32.to_le_bytes());

    data.push(0x08);
    data.push(0x01);
    data.push(0x18);
    data.push(0x05);
    data.push(0x30);
    data.push(20);
    data.push(0x3B);
    data.extend_from_slice(&[0x1F, 0xFF, 0xFF, 0xFF]);
    data.push(0x00);

    data.push(0x00);
    data.extend_from_slice(b"mod\0");
    data.extend_from_slice(b"Foo\0");
    data.extend_from_slice(&[0, 0, 0, 0, 0]);

    let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
    for _ in 0..1024 {
        std::io::Write::write_all(&mut encoder, &[0; 1024]).unwrap();
    }
    let compressed_data = encoder.finish().unwrap();
    data.extend_from_slice(&compressed_data[..20]);

    let path = std::env::temp_dir().join("havoc_jimage_oom.jimage");
    std::fs::write(&path, &data).unwrap();

    ALLOCATED.store(0, Ordering::SeqCst);

    let reader = JImageReader::open(&path).unwrap();
    let _res = reader.read_resource("/mod/Foo");

    let allocated = ALLOCATED.load(Ordering::SeqCst);
    assert!(
        allocated < 10_000_000,
        "Allocated {allocated} bytes! OOM triggered."
    );

    std::fs::remove_file(&path).unwrap();
}
