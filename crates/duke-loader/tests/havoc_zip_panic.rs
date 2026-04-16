//! This module provides tests to verify zip loading panics and crashes.
#[test]
fn havoc_zip_reader_cd_entry_extends_past_bounds() {
    let mut data = vec![0u8; 100];

    // EOCD setup
    let eocd_pos = 100 - 22;
    data[eocd_pos..eocd_pos + 4].copy_from_slice(&0x0605_4b50_u32.to_le_bytes()); // EOCD_SIGNATURE
    data[eocd_pos + 10..eocd_pos + 12].copy_from_slice(&1u16.to_le_bytes()); // count
    data[eocd_pos + 12..eocd_pos + 16].copy_from_slice(&50u32.to_le_bytes()); // cd_size
    data[eocd_pos + 16..eocd_pos + 20].copy_from_slice(&28u32.to_le_bytes()); // cd_offset

    // CD setup
    let cd_pos = 28;
    data[cd_pos..cd_pos + 4].copy_from_slice(&0x0201_4b50_u32.to_le_bytes()); // CD_SIGNATURE
    data[cd_pos + 28..cd_pos + 30].copy_from_slice(&4u16.to_le_bytes()); // filename_len
    data[cd_pos + 30..cd_pos + 32].copy_from_slice(&1000u16.to_le_bytes()); // extra_len = 1000, extends past CD bounds!
    data[cd_pos + 46..cd_pos + 50].copy_from_slice(b"test");

    let res = duke_loader::zip::ZipReader::from_bytes(data);
    assert!(res.is_err());
    if let Err(duke_loader::LoadError::ZipFormat { msg }) = res {
        assert!(
            msg.contains("central directory entry extends past CD bounds")
                || msg.contains("length overflow"),
            "{}",
            msg
        );
    } else {
        panic!("Expected ZipFormat error");
    }
}

#[test]
fn havoc_zip_reader_cd_entry_truncated() {
    let mut data = vec![0u8; 100];

    // EOCD setup
    let eocd_pos = 100 - 22;
    data[eocd_pos..eocd_pos + 4].copy_from_slice(&0x0605_4b50_u32.to_le_bytes()); // EOCD_SIGNATURE
    data[eocd_pos + 10..eocd_pos + 12].copy_from_slice(&1u16.to_le_bytes()); // count
    data[eocd_pos + 12..eocd_pos + 16].copy_from_slice(&10u32.to_le_bytes()); // cd_size = 10!
    data[eocd_pos + 16..eocd_pos + 20].copy_from_slice(&28u32.to_le_bytes()); // cd_offset = 28

    // CD setup
    let cd_pos = 28;
    data[cd_pos..cd_pos + 4].copy_from_slice(&0x0201_4b50_u32.to_le_bytes()); // CD_SIGNATURE

    let res = duke_loader::zip::ZipReader::from_bytes(data);
    assert!(res.is_err());
    if let Err(duke_loader::LoadError::ZipFormat { msg }) = res {
        assert!(msg.contains("central directory entry truncated"), "{}", msg);
    } else {
        panic!("Expected ZipFormat error");
    }
}

#[test]
fn havoc_zip_reader_cd_entry_length_overflow() {
    let mut data = vec![0u8; 100];

    // EOCD setup
    let eocd_pos = 100 - 22;
    data[eocd_pos..eocd_pos + 4].copy_from_slice(&0x0605_4b50_u32.to_le_bytes()); // EOCD_SIGNATURE
    data[eocd_pos + 10..eocd_pos + 12].copy_from_slice(&1u16.to_le_bytes()); // count
    data[eocd_pos + 12..eocd_pos + 16].copy_from_slice(&50u32.to_le_bytes()); // cd_size
    data[eocd_pos + 16..eocd_pos + 20].copy_from_slice(&28u32.to_le_bytes()); // cd_offset

    // CD setup
    let cd_pos = 28;
    data[cd_pos..cd_pos + 4].copy_from_slice(&0x0201_4b50_u32.to_le_bytes()); // CD_SIGNATURE
    data[cd_pos + 28..cd_pos + 30].copy_from_slice(&65535u16.to_le_bytes()); // filename_len
    data[cd_pos + 30..cd_pos + 32].copy_from_slice(&65535u16.to_le_bytes()); // extra_len
    data[cd_pos + 32..cd_pos + 34].copy_from_slice(&65535u16.to_le_bytes()); // comment_len

    let res = duke_loader::zip::ZipReader::from_bytes(data);
    assert!(res.is_err());
    if let Err(duke_loader::LoadError::ZipFormat { msg }) = res {
        assert!(
            msg.contains("central directory entry filename truncated")
                || msg.contains("length overflow"),
            "{}",
            msg
        );
    } else {
        panic!("Expected ZipFormat error");
    }
}

#[test]
fn havoc_zip_reader_local_header_extends_past_archive() {
    let mut data = vec![0u8; 100];

    // EOCD setup
    let eocd_pos = 100 - 22;
    data[eocd_pos..eocd_pos + 4].copy_from_slice(&0x0605_4b50_u32.to_le_bytes());
    data[eocd_pos + 10..eocd_pos + 12].copy_from_slice(&1u16.to_le_bytes());
    data[eocd_pos + 12..eocd_pos + 16].copy_from_slice(&50u32.to_le_bytes());
    data[eocd_pos + 16..eocd_pos + 20].copy_from_slice(&28u32.to_le_bytes());

    // CD setup
    let cd_pos = 28;
    data[cd_pos..cd_pos + 4].copy_from_slice(&0x0201_4b50_u32.to_le_bytes());
    data[cd_pos + 28..cd_pos + 30].copy_from_slice(&4u16.to_le_bytes());
    data[cd_pos + 42..cd_pos + 46].copy_from_slice(&0u32.to_le_bytes()); // offset = 0
    data[cd_pos + 46..cd_pos + 50].copy_from_slice(b"test");

    // Local header setup
    let local_pos = 0;
    data[local_pos..local_pos + 4].copy_from_slice(&0x0403_4b50_u32.to_le_bytes());
    data[local_pos + 26..local_pos + 28].copy_from_slice(&1000u16.to_le_bytes()); // filename_len = 1000, extends past EOF!

    let reader = duke_loader::zip::ZipReader::from_bytes(data).unwrap();
    let res = reader.read_entry("test");
    assert!(res.is_err());
    if let Err(duke_loader::LoadError::ZipFormat { msg }) = res {
        assert!(
            msg.contains("local header extends past archive")
                || msg.contains("data extends past")
                || msg.contains("overflow")
                || msg.contains("truncated"),
            "{}",
            msg
        );
    } else {
        panic!("Expected ZipFormat error");
    }
}

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
    data[local_pos + 26..local_pos + 28].copy_from_slice(&10u16.to_le_bytes());
    // Compressed size = 1000 => data extends past end of archive

    // BUT we need to modify the ZipEntryInfo because we only control the reader bytes.
    // We can't directly change ZipEntryInfo compressed size without a valid CD header.
    data[cd_pos + 20..cd_pos + 24].copy_from_slice(&1000u32.to_le_bytes()); // compressed_size = 1000

    let reader = duke_loader::zip::ZipReader::from_bytes(data).unwrap();
    let res = reader.read_entry("test");
    // Assert that we got an error, and it did not panic
    assert!(res.is_err());

    if let Err(e) = res {
        match e {
            duke_loader::LoadError::ZipFormat { msg } => {
                assert!(
                    msg.contains("data extends past end of archive")
                        || msg.contains("overflow")
                        || msg.contains("truncated")
                );
            }
            _ => panic!("Expected ZipFormat error"),
        }
    }
}

#[test]
fn havoc_zip_reader_local_header_offset_overflow() {
    let mut data = vec![0u8; 100];

    // EOCD setup
    let eocd_pos = 100 - 22;
    data[eocd_pos..eocd_pos + 4].copy_from_slice(&0x0605_4b50_u32.to_le_bytes()); // EOCD_SIGNATURE
    data[eocd_pos + 10..eocd_pos + 12].copy_from_slice(&1u16.to_le_bytes()); // count
    data[eocd_pos + 12..eocd_pos + 16].copy_from_slice(&50u32.to_le_bytes()); // cd_size
    data[eocd_pos + 16..eocd_pos + 20].copy_from_slice(&28u32.to_le_bytes()); // cd_offset

    // CD setup
    let cd_pos = 28;
    data[cd_pos..cd_pos + 4].copy_from_slice(&0x0201_4b50_u32.to_le_bytes()); // CD_SIGNATURE
    data[cd_pos + 28..cd_pos + 30].copy_from_slice(&4u16.to_le_bytes()); // filename_len
    data[cd_pos + 42..cd_pos + 46].copy_from_slice(&u32::MAX.to_le_bytes()); // local_header_offset = u32::MAX
    data[cd_pos + 46..cd_pos + 50].copy_from_slice(b"test");

    let reader = duke_loader::zip::ZipReader::from_bytes(data).unwrap();
    let res = reader.read_entry("test");
    assert!(res.is_err());
    if let Err(duke_loader::LoadError::ZipFormat { msg }) = res {
        assert!(
            msg.contains("local header at offset") && msg.contains("truncated"),
            "{}",
            msg
        );
    } else {
        panic!("Expected ZipFormat error");
    }
}
