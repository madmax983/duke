#![allow(missing_docs)]
use duke_loader::ZipLoader;

fn build_dummy_zip_with_nested(name: &str, nested_content: &[u8]) -> Vec<u8> {
    let mut zip = Vec::new();
    let name_bytes = name.as_bytes();

    // minimal central dir entry
    zip.extend_from_slice(b"PK\x01\x02");
    zip.extend_from_slice(&[0; 16]); // flags, method, etc
    zip.extend_from_slice(&(nested_content.len() as u32).to_le_bytes()); // crc
    zip.extend_from_slice(&(nested_content.len() as u32).to_le_bytes()); // compressed size
    zip.extend_from_slice(&(nested_content.len() as u32).to_le_bytes()); // uncompressed size
    zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes()); // name len
    zip.extend_from_slice(&[0; 12]);
    zip.extend_from_slice(name_bytes);

    let cd_size = (46 + name_bytes.len()) as u32;

    // local file header
    zip.extend_from_slice(b"PK\x03\x04");
    zip.extend_from_slice(&[0; 10]);
    zip.extend_from_slice(&(nested_content.len() as u32).to_le_bytes()); // crc
    zip.extend_from_slice(&(nested_content.len() as u32).to_le_bytes()); // compressed size
    zip.extend_from_slice(&(nested_content.len() as u32).to_le_bytes()); // uncompressed size
    zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes()); // name len
    zip.extend_from_slice(&[0; 2]); // extra field len
    zip.extend_from_slice(name_bytes);
    zip.extend_from_slice(nested_content);

    // EOCD
    zip.extend_from_slice(b"PK\x05\x06");
    zip.extend_from_slice(&[0; 4]); // disk numbers
    zip.extend_from_slice(&1u16.to_le_bytes()); // entry count
    zip.extend_from_slice(&1u16.to_le_bytes());
    zip.extend_from_slice(&cd_size.to_le_bytes()); // size
    zip.extend_from_slice(&0u32.to_le_bytes()); // offset
    zip.extend_from_slice(&[0; 2]); // comment len

    zip
}

#[test]
fn test_zip_bomb_infinite_recursion() {
    // Level 0: empty zip (or just any valid zip)
    let current_zip = build_dummy_zip_with_nested("empty", b"");
    let current_zip = build_dummy_zip_with_nested("BOOT-INF/lib/nested.jar", &current_zip);
    let current_zip = build_dummy_zip_with_nested("BOOT-INF/lib/nested.jar", &current_zip);
    let current_zip = build_dummy_zip_with_nested("BOOT-INF/lib/nested.jar", &current_zip);

    let path = std::env::temp_dir().join("havoc_bomb.jar");
    std::fs::write(&path, &current_zip).unwrap();

    let _loader = ZipLoader::open(&path);

    std::fs::remove_file(&path).unwrap();
}
