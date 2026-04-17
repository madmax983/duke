//! Tests for Havoc ZIP OOM issues

#[test]
fn havoc_test_oom() {
    let bad_data = vec![
        // EOCD signature
        0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0xff, 0xff, // 65535 entries
        0xff, 0xff, 0, 0, 0, 0, // cd size
        0, 0, 0, 0, // offset
        0, 0,
    ];
    let res = duke_loader::zip::ZipReader::from_bytes(bad_data);
    assert!(res.is_err());
}
