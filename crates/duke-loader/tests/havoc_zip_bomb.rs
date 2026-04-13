#![allow(clippy::unreadable_literal, clippy::cast_possible_truncation)]
use duke_loader::{ClassLoader, ZipLoader};
use flate2::Compression;
use flate2::write::DeflateEncoder;
use std::io::Write;

#[test]
fn havoc_zip_bomb() {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    let chunk = vec![0u8; 1024 * 1024]; // 1MB of zeros
    for _ in 0..10 {
        encoder.write_all(&chunk).unwrap(); // 10MB
    }
    let compressed_data = encoder.finish().unwrap();

    let name_class = "bomb.class";
    let uncompressed_size: u32 = 0xFFFFFFFF; // Try to OOM with 4GB size claim
    let mut data = Vec::new();

    let compressed_size = compressed_data.len() as u32;
    let filename_len = name_class.len() as u16;
    let crc32: u32 = crc32fast::hash(&vec![0u8; 10 * 1024 * 1024]);

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

    let central_dir_size = (data.len() as u32) - central_header_offset;

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
    let temp_path = temp_dir.join("bomb.zip");
    std::fs::write(&temp_path, &data).unwrap();

    let loader = ZipLoader::open(&temp_path).unwrap();

    // We expect it to either OOM or if we add limits, it will return an error!
    let result = loader.find_class("bomb");

    let _ = std::fs::remove_file(&temp_path);

    // The test passes if we get an Error instead of OOM!
    assert!(
        result.is_err(),
        "Zip Bomb vulnerability! Decompression succeeded but should have been rejected or bounded."
    );
}
