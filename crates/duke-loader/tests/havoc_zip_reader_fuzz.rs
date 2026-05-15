#![allow(missing_docs)]

use duke_loader::ZipReader;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn fuzz_zip_reader_from_bytes(data in any::<Vec<u8>>()) {
        let _ = ZipReader::from_bytes(data);
    }
}
