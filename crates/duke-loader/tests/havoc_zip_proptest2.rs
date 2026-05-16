#![allow(missing_docs)]

use duke_loader::ZipReader;
use proptest::prelude::*;

proptest! {
    #[test]
    fn havoc_zip_reader_read_entry_info_panic(
        name in ".*",
    ) {
        // Build a dummy zip with a valid CD but the dummy fields
        let mut zip_data = Vec::new();

        // EOCD
        zip_data.extend_from_slice(&0x0605_4b50_u32.to_le_bytes()); // EOCD signature
        zip_data.extend_from_slice(&0_u16.to_le_bytes()); // disk number
        zip_data.extend_from_slice(&0_u16.to_le_bytes()); // disk where CD starts
        zip_data.extend_from_slice(&0_u16.to_le_bytes()); // number of CD entries on this disk
        zip_data.extend_from_slice(&0_u16.to_le_bytes()); // total number of CD entries
        zip_data.extend_from_slice(&0_u32.to_le_bytes()); // size of CD
        zip_data.extend_from_slice(&0_u32.to_le_bytes()); // offset of start of CD
        zip_data.extend_from_slice(&0_u16.to_le_bytes()); // comment length

        if let Ok(reader) = ZipReader::from_bytes(zip_data) {
             let _ = reader.read_entry(&name);
        }
    }
}
