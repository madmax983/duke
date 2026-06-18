#![allow(missing_docs)]
use proptest::prelude::*;
use duke_loader::JImageReader;
use std::io::Write;

proptest! {
    #[test]
    fn test_jimage_open_fuzzing(
        magic in any::<u32>(),
        version in any::<u32>(),
        flags in any::<u32>(),
        resource_count in any::<u32>(),
        table_length in any::<u32>(),
        locations_size in any::<u32>(),
        strings_size in any::<u32>(),
        index_hash_multiplier in any::<i32>(),
        index_shift in any::<u32>(),
        junk in prop::collection::vec(any::<u8>(), 0..10_000)
    ) {
        let mut file = tempfile::NamedTempFile::new().unwrap();

        // Write structured header
        file.write_all(&magic.to_le_bytes()).unwrap();
        file.write_all(&version.to_le_bytes()).unwrap();
        file.write_all(&flags.to_le_bytes()).unwrap();
        file.write_all(&resource_count.to_le_bytes()).unwrap();
        file.write_all(&table_length.to_le_bytes()).unwrap();
        file.write_all(&locations_size.to_le_bytes()).unwrap();
        file.write_all(&strings_size.to_le_bytes()).unwrap();
        file.write_all(&index_hash_multiplier.to_le_bytes()).unwrap();
        file.write_all(&index_shift.to_le_bytes()).unwrap();

        file.write_all(&junk).unwrap();

        let _ = JImageReader::open(file.path());
    }
}
