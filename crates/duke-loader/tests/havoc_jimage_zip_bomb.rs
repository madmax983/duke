use duke_loader::{JImageReader, LoadError};
use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::io::Write;

#[test]
fn havoc_crash_jimage_zip_bomb() {
    let zeroes = vec![0u8; 300 * 1024 * 1024]; // 300MB of zeros
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::none());
    encoder.write_all(&zeroes).unwrap();
    let mut compressed = encoder.finish().unwrap();

    let mut bad_data: Vec<u8> = vec![
        0xDA, 0xDA, 0xFE, 0xCA, // Magic (CAFE_DADA, le)
        0x00, 0x00, 0x01, 0x00, // Version (0001_0000, le)
        0x00, 0x00, 0x00, 0x00, // Flags
        0x01, 0x00, 0x00, 0x00, // Resource count = 1
        0x01, 0x00, 0x00, 0x00, // Table length = 1
        0x38, 0x00, 0x00, 0x00, // Locations size = 56
        0x20, 0x00, 0x00, 0x00, // Strings size = 32
    ];
    // redirect
    bad_data.extend(&[0x00, 0x00, 0x00, 0x00]);
    // offsets
    bad_data.extend(&[0x00, 0x00, 0x00, 0x00]);

    let compressed_len = compressed.len() as u64;
    let uncompressed_len = zeroes.len() as u64;

    // locations
    bad_data.push(0x08 | 0x07); // MODULE (kind 1), length 8
    bad_data.extend(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]);
    bad_data.push(0x18 | 0x07); // BASE (kind 3), length 8
    bad_data.extend(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05]);
    bad_data.push(0x28 | 0x07); // OFFSET (kind 5), length 8
    bad_data.extend(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    bad_data.push(0x30 | 0x07); // COMPRESSED (kind 6), length 8
    bad_data.extend(&compressed_len.to_be_bytes());
    bad_data.push(0x38 | 0x07); // UNCOMPRESSED (kind 7), length 8
    bad_data.extend(&uncompressed_len.to_be_bytes());
    bad_data.push(0x00); // END
    bad_data.extend(&[0x00; 10]); // padding

    // strings
    bad_data.extend(&[0x00, b'm', b'o', b'd', 0x00, b'b', b'a', b's', b'e', 0x00]);
    bad_data.extend(&[0x00; 22]);

    bad_data.append(&mut compressed);

    let file_path = std::env::temp_dir().join(format!("bad_jimage_bomb_{}.jimage", std::process::id()));
    std::fs::write(&file_path, &bad_data).unwrap();
    let reader = JImageReader::open(&file_path).unwrap();

    // Attempt to read the bomb
    let res = reader.read_resource("/mod/base");

    std::fs::remove_file(&file_path).unwrap();

    match res {
        Ok(_) => panic!("Zip Bomb decompressed completely instead of erroring! Memory vulnerability exists."),
        Err(LoadError::JImageFormat { msg }) if msg.contains("exceeds limit") => {
            // Expected failure after our fix
        }
        Err(e) => panic!("Expected JImageFormat limit error, got {:?}", e),
    }
}
