#![allow(missing_docs)]

#[test]
fn havoc_jimage_oob_read_into_strings() {
    let mut data = vec![0u8; 28];
    data[0..4].copy_from_slice(&0xCAFE_DADA_u32.to_le_bytes()); // magic
    data[4..8].copy_from_slice(&0x0001_0000_u32.to_le_bytes()); // version
    data[8..12].copy_from_slice(&0_u32.to_le_bytes()); // flags
    data[12..16].copy_from_slice(&1_u32.to_le_bytes()); // resource_count
    data[16..20].copy_from_slice(&1_u32.to_le_bytes()); // table_length = 1

    let locations_size: u32 = 5;
    let strings_size: u32 = 12;

    data[20..24].copy_from_slice(&locations_size.to_le_bytes());
    data[24..28].copy_from_slice(&strings_size.to_le_bytes());

    data.extend_from_slice(&0_u32.to_le_bytes()); // redirect: [i32; 1]
    data.extend_from_slice(&0_u32.to_le_bytes()); // offsets: [u32; 1]

    // locations: (5 bytes)
    // MODULE: kind 1. len=1 -> hdr=8. Val=1 ("M")
    data.push(8);
    data.push(1);

    // BASE: kind 3. len=1 -> hdr=24. Val=3 ("B")
    data.push(24);
    data.push(3);

    // UNCOMPRESSED: kind 7. len=8 -> hdr=63.
    data.push(63);
    // locations table ends here (5 bytes).

    // The UNCOMPRESSED payload (8 bytes) will be read entirely from the string table!
    // strings table: (12 bytes)
    data.push(0x00); // 0: \0
    data.push(b'M'); // 1: M
    data.push(0x00); // 2: \0
    data.push(b'B'); // 3: B
    data.push(0x00); // 4: \0
    data.push(0x55); // 5
    data.push(0x66); // 6
    data.push(0x77); // 7
    data.push(0x88); // 8
    data.push(0x99); // 9
    data.push(0xAA); // 10
    data.push(0xBB); // 11

    let tmp = std::env::temp_dir().join("havoc_jimage_oob_read_safe.jimage");
    std::fs::write(&tmp, &data).unwrap();

    let reader = duke_loader::JImageReader::open(&tmp).unwrap();
    let _ = std::fs::remove_file(&tmp);

    let info = reader.find_resource("/M/B");
    assert!(
        info.is_none(),
        "Resource should not be fully parsed if bounds check is correct"
    );
}
