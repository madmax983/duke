use duke_loader::jimage::JImageReader;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_jimage_fuzz_str_overflow() {
    let mut file = NamedTempFile::new().unwrap();
    let mut data = vec![
        // magic
        0xDA, 0xDA, 0xFE, 0xCA, // version
        0x00, 0x00, 0x01, 0x00, // flags
        0x00, 0x00, 0x00, 0x00, // resource_count
        0x01, 0x00, 0x00, 0x00, // table_length
        0x01, 0x00, 0x00, 0x00, // locations_size
        0x10, 0x00, 0x00, 0x00, // 16 bytes
        // strings_size
        0x00, 0x00, 0x00, 0x00,
    ];
    // tl = 1. offset = HEADER_SIZE + 1*8 = 36.
    // ls = 16. str_offset = 36 + 16 = 52.
    // Let's add padding up to locations (36 bytes)
    while data.len() < 36 {
        data.push(0);
    }
    // Location entry:
    // We want ATTR_MODULE (kind = 1) and len = 8
    // hdr = (1 << 3) | (8 - 1) = 8 | 7 = 15 = 0x0F
    data.push(0x0F);

    // We want idx_usize = usize::MAX - str_offset + 55
    // usize::MAX = 0xFFFFFFFFFFFFFFFF
    // str_offset = 52
    // idx = 0xFFFFFFFFFFFFFFFF - 52 + 55 = 0xFFFFFFFFFFFFFFFF + 3 -> overflow
    // Wait, let's just use 0xFFFFFFFFFFFFFFFF - 52 + 50 = 0xFFFFFFFFFFFFFFFE.
    // This will result in `start` = 52 + 0xFFFFFFFFFFFFFFFE = 50.
    // data.len() will be around 60. So `start >= data.len()` is false!
    let idx: u64 = u64::MAX - 52 + 50;
    for &b in &idx.to_be_bytes() {
        data.push(b);
    }

    // Now ATTR_UNCOMPRESSED (kind = 7) len = 1
    // hdr = (7 << 3) | 0 = 56 = 0x38
    data.push(0x38);
    data.push(0x01); // value 1

    // Pad to base (ATTR_BASE kind=3) len=1
    data.push(0x18);
    data.push(0x01); // base > 0 so it's not skipped

    // End (ATTR_END)
    data.push(0x00);

    // Padding to 60 bytes
    while data.len() < 60 {
        data.push(0);
    }

    file.write_all(&data).unwrap();
    let _ = JImageReader::open(file.path());
}
