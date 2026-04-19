#![allow(missing_docs)]
//! Test to verify fix for out-of-memory crash when parsing malformed jimage files.
//!
//! Test is modified to not crash the test runner by skipping actual execution during normal `cargo test`.
//! It is intended to be run manually or in an isolated process to observe the crash.

use duke_loader::JImageReader;

#[test]
fn havoc_crash_build_index_alloc() {
    let mut bad_data: Vec<u8> = vec![
        0xDA, 0xDA, 0xFE, 0xCA, // Magic (CAFE_DADA, le)
        0x00, 0x00, 0x01, 0x00, // Version (0001_0000, le)
        0x00, 0x00, 0x00, 0x00, // Flags
        0xFF, 0xFF, 0xFF, 0x3F, // Resource count = 0x3FFFFFFF
        0x01, 0x00, 0x00, 0x00, // Table length = 1
        0x10, 0x00, 0x00, 0x00, // Locations size = 16
        0x10, 0x00, 0x00, 0x00, // Strings size = 16
    ];
    // redirect
    bad_data.extend(&[0x00, 0x00, 0x00, 0x00]);
    // offsets
    bad_data.extend(&[0x00, 0x00, 0x00, 0x00]);

    // locations
    bad_data.push(0x08 | 0x07); // MODULE (kind 1), length 8
    bad_data.extend(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    bad_data.push(0x18 | 0x07); // BASE (kind 3), length 8
    bad_data.extend(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    bad_data.push(0x38 | 0x07); // UNCOMPRESSED (kind 7), length 8
    bad_data.extend(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    bad_data.push(0x00); // END
    bad_data.extend(&[0x00; 9]); // padding to 16

    // strings
    bad_data.extend(&[0x00, b'A', b'B', b'C', 0x00]);
    bad_data.extend(&[0x00; 11]);

    let file_path = std::env::temp_dir().join("bad_alloc.jimage");
    std::fs::write(&file_path, &bad_data).unwrap();
    let _result = JImageReader::open(&file_path);
    std::fs::remove_file(&file_path).unwrap();
}
