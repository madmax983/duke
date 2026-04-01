use duke_loader::ZipLoader;
use std::fs;

#[test]
fn havoc_zip_overflow() {
    let mut zip = vec![0u8; 120];

    // Local file header signature
    zip[0..4].copy_from_slice(&0x04034b50u32.to_le_bytes());
    // file name length
    let filename_len: u16 = 65535;
    zip[26..28].copy_from_slice(&filename_len.to_le_bytes());
    // extra field length
    let extra_len: u16 = 65535;
    zip[28..30].copy_from_slice(&extra_len.to_le_bytes());

    // write central directory
    let cd_offset = 30; // pretend CD starts right after
    let cd_sig: u32 = 0x02014b50;
    zip[cd_offset..cd_offset+4].copy_from_slice(&cd_sig.to_le_bytes());

    // compressed size
    let cd_compressed_size: u32 = 0xffffffff;
    zip[cd_offset+20..cd_offset+24].copy_from_slice(&cd_compressed_size.to_le_bytes());
    // uncompressed size
    zip[cd_offset+24..cd_offset+28].copy_from_slice(&cd_compressed_size.to_le_bytes());
    // file name length
    let cd_filename_len: u16 = 7;
    zip[cd_offset+28..cd_offset+30].copy_from_slice(&cd_filename_len.to_le_bytes());
    // file name
    zip[cd_offset+46..cd_offset+53].copy_from_slice(b"a.class");

    // EOCD
    let eocd_offset = cd_offset + 53;
    let eocd_sig: u32 = 0x06054b50;
    zip[eocd_offset..eocd_offset+4].copy_from_slice(&eocd_sig.to_le_bytes());
    // total number of entries in the central directory
    zip[eocd_offset+10..eocd_offset+12].copy_from_slice(&1u16.to_le_bytes());
    // size of the central directory
    zip[eocd_offset+12..eocd_offset+16].copy_from_slice(&53u32.to_le_bytes());
    // offset of start of central directory with respect to the starting disk number
    zip[eocd_offset+16..eocd_offset+20].copy_from_slice(&(cd_offset as u32).to_le_bytes());

    let tmp = std::env::temp_dir().join("havoc_zip.jar");
    fs::write(&tmp, &zip).unwrap();
    if let Ok(loader) = ZipLoader::open(&tmp) {
        use duke_loader::ClassLoader;
        // Even if find_class fails for other formatting reasons, it should not panic
        // due to out-of-bounds slice access caused by integer overflow
        let _ = loader.find_class("a");
    }
    let _ = fs::remove_file(&tmp);
}
