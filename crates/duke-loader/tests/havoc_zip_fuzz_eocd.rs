use duke_loader::ZipLoader;

#[test]
fn fuzz_zip_loader_huge_alloc2() {
    let mut zip_content = vec![0; 4096];

    // EOCD signature
    zip_content[4096 - 22..4096 - 18].copy_from_slice(&0x0605_4B50_u32.to_le_bytes());
    // CD entries count = 0xFFFF (65535)
    zip_content[4096 - 22 + 10..4096 - 22 + 12].copy_from_slice(&0xFFFF_u16.to_le_bytes());
    // CD size
    zip_content[4096 - 22 + 12..4096 - 22 + 16].copy_from_slice(&0_u32.to_le_bytes());
    // CD offset
    zip_content[4096 - 22 + 16..4096 - 22 + 20].copy_from_slice(&(4096 - 22_u32).to_le_bytes());

    let zip_path = std::env::temp_dir().join("havoc_zip_loader_eocd.zip");
    std::fs::write(&zip_path, &zip_content).unwrap();
    let _ = ZipLoader::open(&zip_path);
    std::fs::remove_file(&zip_path).unwrap();
}
