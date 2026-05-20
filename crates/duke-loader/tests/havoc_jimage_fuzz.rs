#![allow(missing_docs)]

use std::io::Write;
use std::fs::File;
use std::panic;

#[test]
fn test_havoc_jimage_oob_panic() {
    let temp_path = std::env::temp_dir().join("havoc_jimage_fuzz_bounds.jimage");

    // We want `read_u32_le` to panic. It was vulnerable when reading the index redirect/offsets.
    // However, the function `parse_header` already checks `if data.len() < HEADER_SIZE`.
    // Let's create an invalid header that triggers out of bounds panic if we restore the broken code.
    let mut data = vec![0u8; 32];
    data[0..4].copy_from_slice(&0xCA_FE_DA_DA_u32.to_le_bytes()); // magic
    data[4..8].copy_from_slice(&0x0001_0000_u32.to_le_bytes()); // version
    data[8..12].copy_from_slice(&0_u32.to_le_bytes()); // flags
    data[12..16].copy_from_slice(&1_u32.to_le_bytes()); // resource_count
    data[16..20].copy_from_slice(&1_u32.to_le_bytes()); // table_length
    data[20..24].copy_from_slice(&0_u32.to_le_bytes()); // locations_size
    data[24..28].copy_from_slice(&0_u32.to_le_bytes()); // strings_size

    data.extend(vec![0u8; 10]);

    let mut file = File::create(&temp_path).unwrap();
    file.write_all(&data).unwrap();
    drop(file);

    // If the vulnerability exists, this call will panic.
    // If it's fixed properly, it will return an error (or successfully parse a garbage file but not panic).
    let result = panic::catch_unwind(|| {
        let _ = duke_loader::JImageReader::open(&temp_path);
    });

    assert!(result.is_ok(), "The JImage parser panicked! It should handle OOB gracefully.");

    let _ = std::fs::remove_file(temp_path);
}
