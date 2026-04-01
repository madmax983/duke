use duke_loader::zip::ZipReader;

#[test]
fn havoc_zip_overflow_fuzz() {
    let mut data = vec![0u8; 100];
    let eocd_pos = data.len() - 22;
    data[eocd_pos..eocd_pos+4].copy_from_slice(&0x0605_4b50_u32.to_le_bytes());
    data[eocd_pos+10..eocd_pos+12].copy_from_slice(&1_u16.to_le_bytes());
    data[eocd_pos+12..eocd_pos+16].copy_from_slice(&46_u32.to_le_bytes());
    let cd_offset = 0_usize;
    data[eocd_pos+16..eocd_pos+20].copy_from_slice(&(cd_offset as u32).to_le_bytes());

    // CD entry
    data[cd_offset..cd_offset+4].copy_from_slice(&0x0201_4b50_u32.to_le_bytes());

    let compressed_size = u32::MAX;
    data[cd_offset+20..cd_offset+24].copy_from_slice(&compressed_size.to_le_bytes());

    let local_header_offset = u32::MAX; // Just past the CD
    data[cd_offset+42..cd_offset+46].copy_from_slice(&local_header_offset.to_le_bytes());

    // This used to panic due to integer overflow, but now returns an error gracefully.
    let reader = ZipReader::from_bytes(data).unwrap();
    let _ = reader.read_entry("");
}
