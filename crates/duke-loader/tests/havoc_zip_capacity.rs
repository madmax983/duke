#![allow(clippy::unreadable_literal, clippy::cast_possible_truncation)]
use duke_loader::{ClassLoader, ZipLoader};
use flate2::Compression;
use flate2::write::DeflateEncoder;
use std::io::Write;

#[test]
fn havoc_zip_capacity() {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    // Create actual 256MB of zeros to hit the second error block, but it might OOM us too.
    // Wait, the test already covers the `info.uncompressed_size > MAX_SIZE` block.
    // What about `if buf.len() as u64 == MAX_SIZE`?
    // To trigger that, the uncompressed_size has to be <= MAX_SIZE, but the decompressed payload actually hits MAX_SIZE.
    // Let's create an archive where `uncompressed_size` is spoofed to something small (like 100), but the actual payload is 256MB.

    // We can just construct a small chunk that compresses well, but uncompresses to > 256MB,
    // and spoof its uncompressed_size to 100.
    let chunk = vec![0u8; 1024 * 1024]; // 1MB
    for _ in 0..257 {
        encoder.write_all(&chunk).unwrap(); // 257MB
    }
    let compressed_data = encoder.finish().unwrap();

    let name_class = "capacity.class";
    let uncompressed_size: u32 = 100; // Spoof small size
    let mut data = Vec::new();

    let compressed_size = compressed_data.len() as u32;
    let filename_len = name_class.len() as u16;
    let crc32: u32 = 0;

    // LOCAL HEADER
    data.extend_from_slice(&0x04034b50u32.to_le_bytes());
    data.extend_from_slice(&20u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&8u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&crc32.to_le_bytes());
    data.extend_from_slice(&compressed_size.to_le_bytes());
    data.extend_from_slice(&uncompressed_size.to_le_bytes());
    data.extend_from_slice(&filename_len.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(name_class.as_bytes());
    data.extend_from_slice(&compressed_data);

    let central_header_offset = data.len() as u32;

    // CENTRAL DIRECTORY HEADER
    data.extend_from_slice(&0x02014b50u32.to_le_bytes());
    data.extend_from_slice(&20u16.to_le_bytes());
    data.extend_from_slice(&20u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&8u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&crc32.to_le_bytes());
    data.extend_from_slice(&compressed_size.to_le_bytes());
    data.extend_from_slice(&uncompressed_size.to_le_bytes());
    data.extend_from_slice(&filename_len.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(name_class.as_bytes());

    let central_dir_size = u32::try_from(data.len()).unwrap() - central_header_offset;

    // END OF CENTRAL DIRECTORY
    data.extend_from_slice(&0x06054b50u32.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&central_dir_size.to_le_bytes());
    data.extend_from_slice(&central_header_offset.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());

    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("capacity.zip");
    std::fs::write(&temp_path, &data).unwrap();

    let loader = ZipLoader::open(&temp_path).unwrap();
    let result = loader.find_class("capacity");

    let _ = std::fs::remove_file(&temp_path);

    assert!(result.is_err());
}
