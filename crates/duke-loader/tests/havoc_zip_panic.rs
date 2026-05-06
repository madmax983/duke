#![allow(missing_docs)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::cast_possible_truncation)]
use duke_loader::ZipReader;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_zip_read_entry_info_fuzz(
        mut data in proptest::collection::vec(any::<u8>(), 30..1000),
        filename_len in any::<u16>(),
        extra_len in any::<u16>()
    ) {
        data[0..4].copy_from_slice(&0x0403_4b50_u32.to_le_bytes()); // LOCAL_SIGNATURE
        data[26..28].copy_from_slice(&filename_len.to_le_bytes());
        data[28..30].copy_from_slice(&extra_len.to_le_bytes());

        let mut cd_data = vec![];
        cd_data.extend_from_slice(&0x0201_4b50_u32.to_le_bytes()); // CD_SIGNATURE
        cd_data.extend_from_slice(&20_u16.to_le_bytes());
        cd_data.extend_from_slice(&20_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u16.to_le_bytes()); // METHOD_STORED
        cd_data.extend_from_slice(&0_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u32.to_le_bytes()); // crc
        cd_data.extend_from_slice(&0_u32.to_le_bytes()); // compressed size
        cd_data.extend_from_slice(&0_u32.to_le_bytes()); // uncompressed size
        cd_data.extend_from_slice(&4_u16.to_le_bytes()); // name_len
        cd_data.extend_from_slice(&0_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u32.to_le_bytes());
        cd_data.extend_from_slice(&0_u32.to_le_bytes()); // local_offset
        cd_data.extend_from_slice(b"test");

        let cd_offset = data.len() as u32;
        data.extend_from_slice(&cd_data);

        let cd_size = (data.len() as u32) - cd_offset;

        // EOCD
        data.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&1_u16.to_le_bytes());
        data.extend_from_slice(&1_u16.to_le_bytes());
        data.extend_from_slice(&cd_size.to_le_bytes());
        data.extend_from_slice(&cd_offset.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());

        if let Ok(reader) = ZipReader::from_bytes(data) {
            let _ = reader.read_entry("test");
        }
    }
}
