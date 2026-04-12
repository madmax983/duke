#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
use duke_loader::zip::ZipReader;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_zip_reader_read_entry_info_no_oom(
        uncompressed_size in any::<u64>()
    ) {
        let mut data = vec![0; 30];
        data[0..4].copy_from_slice(&0x0403_4b50_u32.to_le_bytes()); // LOCAL_SIGNATURE

        let mut cd_data = vec![];
        cd_data.extend_from_slice(&0x0201_4b50_u32.to_le_bytes()); // CD_SIGNATURE
        cd_data.extend_from_slice(&20_u16.to_le_bytes());
        cd_data.extend_from_slice(&20_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u16.to_le_bytes());
        cd_data.extend_from_slice(&8_u16.to_le_bytes()); // METHOD_DEFLATED
        cd_data.extend_from_slice(&0_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u32.to_le_bytes()); // crc
        cd_data.extend_from_slice(&0_u32.to_le_bytes()); // compressed size
        cd_data.extend_from_slice(&((uncompressed_size % (u32::MAX as u64 + 1)) as u32).to_le_bytes()); // uncompressed size
        cd_data.extend_from_slice(&8_u16.to_le_bytes()); // name_len
        cd_data.extend_from_slice(&0_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u16.to_le_bytes());
        cd_data.extend_from_slice(&0_u32.to_le_bytes());
        cd_data.extend_from_slice(&0_u32.to_le_bytes()); // local_offset
        cd_data.extend_from_slice(b"fuzz.txt");

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

        let reader = ZipReader::from_bytes(data);
        if let Ok(reader) = reader {
            let _ = reader.read_entry("fuzz.txt");
        }
    }
}
