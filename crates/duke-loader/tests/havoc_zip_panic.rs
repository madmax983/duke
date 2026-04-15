#[test]
fn havoc_zip_reader_read_entry_info_panic() {
    let mut data = vec![0u8; 100];

    // EOCD setup (to parse properly)
    let eocd_pos = 100 - 22;
    data[eocd_pos..eocd_pos + 4].copy_from_slice(&0x0605_4b50_u32.to_le_bytes()); // EOCD_SIGNATURE
    data[eocd_pos + 10..eocd_pos + 12].copy_from_slice(&1u16.to_le_bytes()); // count
    data[eocd_pos + 12..eocd_pos + 16].copy_from_slice(&50u32.to_le_bytes()); // cd_size
    data[eocd_pos + 16..eocd_pos + 20].copy_from_slice(&28u32.to_le_bytes()); // cd_offset

    // CD setup
    let cd_pos = 28;
    data[cd_pos..cd_pos + 4].copy_from_slice(&0x0201_4b50_u32.to_le_bytes()); // CD_SIGNATURE
    data[cd_pos + 28..cd_pos + 30].copy_from_slice(&4u16.to_le_bytes()); // filename_len
    data[cd_pos + 42..cd_pos + 46].copy_from_slice(&0u32.to_le_bytes()); // local_header_offset
    data[cd_pos + 46..cd_pos + 50].copy_from_slice(b"test");

    // Local header setup
    let local_pos = 0;
    data[local_pos..local_pos + 4].copy_from_slice(&0x0403_4b50_u32.to_le_bytes()); // LOCAL_SIGNATURE
    data[local_pos + 26..local_pos + 28].copy_from_slice(&65535u16.to_le_bytes()); // local filename_len = 65535!

    let reader = duke_loader::zip::ZipReader::from_bytes(data).unwrap();
    let res = reader.read_entry("test");
    // Assert that we got an error, and it did not panic
    assert!(res.is_err());

    if let Err(e) = res {
        match e {
            duke_loader::LoadError::ZipFormat { msg } => {
                assert!(
                    msg.contains("data extends past end of archive") || msg.contains("overflow")
                );
            }
            _ => panic!("Expected ZipFormat error"),
        }
    }
}
