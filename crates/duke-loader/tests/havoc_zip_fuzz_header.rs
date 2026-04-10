use duke_loader::ZipLoader;

#[test]
fn fuzz_zip_loader_huge_alloc() {
    let zip_content = vec![
        0x50, 0x4B, 0x03, 0x04, // Local file header signature
        0x14, 0x00, // Version needed to extract
        0x00, 0x00, // General purpose bit flag
        0x00, 0x00, // Compression method (Store)
        0x00, 0x00, // Last mod file time
        0x00, 0x00, // Last mod file date
        0x00, 0x00, 0x00, 0x00, // CRC-32
        0xFF, 0xFF, 0xFF, 0x0F, // Compressed size (huge)
        0xFF, 0xFF, 0xFF, 0x7F, // Uncompressed size (huge, almost 2GB)
        0x01, 0x00, // File name length
        0x00, 0x00, // Extra field length
        b'a', // File name
        b'b', // Content (truncated)
    ];

    let zip_path = std::env::temp_dir().join("havoc_zip_loader_huge.zip");
    std::fs::write(&zip_path, &zip_content).unwrap();
    let _ = ZipLoader::open(&zip_path);
    std::fs::remove_file(&zip_path).unwrap();
}
