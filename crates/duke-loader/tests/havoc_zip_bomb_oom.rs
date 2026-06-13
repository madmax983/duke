#![allow(clippy::items_after_statements)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::unnecessary_cast)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
const OOM_LIMIT: usize = 1024 * 1024 * 300; // 300 MB

struct TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let current = ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
        if current + layout.size() > OOM_LIMIT {
            eprintln!("💥 OOM triggered! Attempted to allocate {} bytes, total is {}", layout.size(), current + layout.size());
            std::process::abort();
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

#[test]
fn test_zip_bomb() {
    let mut zip = Vec::new();
    let uncompressed_size: u32 = 1024 * 1024 * 255;

    use flate2::Compression;
    use flate2::write::DeflateEncoder;
    use std::io::Write;

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
    let chunk = vec![0; 1024 * 1024]; // 1MB
    for _ in 0..255 {
        encoder.write_all(&chunk).unwrap();
    }
    let compressed = encoder.finish().unwrap();

    let crc: u32 = 0x12345678; // fake CRC

    let name = "bomb.txt";
    let name_bytes = name.as_bytes();

    zip.extend_from_slice(&0x04034b50u32.to_le_bytes());
    zip.extend_from_slice(&20u16.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip.extend_from_slice(&8u16.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip.extend_from_slice(&crc.to_le_bytes());
    zip.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
    zip.extend_from_slice(&(uncompressed_size as u32).to_le_bytes());
    zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip.extend_from_slice(name_bytes);
    zip.extend_from_slice(&compressed);

    let cd_offset = zip.len() as u32;

    zip.extend_from_slice(&0x02014b50u32.to_le_bytes());
    zip.extend_from_slice(&20u16.to_le_bytes());
    zip.extend_from_slice(&20u16.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip.extend_from_slice(&8u16.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip.extend_from_slice(&crc.to_le_bytes());
    zip.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
    zip.extend_from_slice(&(uncompressed_size as u32).to_le_bytes());
    zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip.extend_from_slice(&0u32.to_le_bytes());
    zip.extend_from_slice(&0u32.to_le_bytes());
    zip.extend_from_slice(name_bytes);

    let cd_size = zip.len() as u32 - cd_offset;

    zip.extend_from_slice(&0x06054b50u32.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip.extend_from_slice(&1u16.to_le_bytes());
    zip.extend_from_slice(&1u16.to_le_bytes());
    zip.extend_from_slice(&cd_size.to_le_bytes());
    zip.extend_from_slice(&cd_offset.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());

    let reader = duke_loader::ZipReader::from_bytes(zip).expect("Failed to create ZipReader");

    // The memory allocator limit is 300MB.
    // Without compression ratio protection, extracting 255MB would trigger OOM.
    // With protection, this should fail with a ZipFormat error.
    let entry = reader.read_entry("bomb.txt");
    assert!(matches!(entry, Err(duke_loader::Error::ZipFormat { msg }) if msg.contains("compression ratio")));
}
