#![allow(missing_docs)]

use duke_loader::JImageReader;
use proptest::prelude::*;
use std::fs::File;
use std::io::Write;

proptest! {
    #[test]
    fn havoc_jimage_read_resource_panic(
        name in ".*",
        resource_count in any::<u32>(),
        table_length in any::<u32>(),
        locations_size in any::<u32>(),
        strings_size in any::<u32>(),
    ) {
        // Build a dummy jimage reader
        let mut data = Vec::new();
        // Magic
        data.extend_from_slice(&0xcafe_d00d_u32.to_le_bytes());
        // Version
        data.extend_from_slice(&1_u32.to_le_bytes());
        // flags
        data.extend_from_slice(&0_u32.to_le_bytes());
        // resource count
        data.extend_from_slice(&resource_count.to_le_bytes());
        // table length
        data.extend_from_slice(&table_length.to_le_bytes());
        // locations size
        data.extend_from_slice(&locations_size.to_le_bytes());
        // strings size
        data.extend_from_slice(&strings_size.to_le_bytes());

        let path = std::env::temp_dir().join(format!("havoc_jimage_proptest2_{}.jimage", uuid::Uuid::new_v4()));
        let mut file = File::create(&path).unwrap();
        file.write_all(&data).unwrap();
        file.sync_all().unwrap();

        if let Ok(reader) = JImageReader::open(&path) {
             let _ = reader.read_resource(&name);
        }

        let _ = std::fs::remove_file(&path);
    }
}
