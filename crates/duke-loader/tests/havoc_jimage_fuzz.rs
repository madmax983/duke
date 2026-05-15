#![allow(missing_docs)]

use duke_loader::JImageReader;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn fuzz_jimage_parse_header(data in any::<Vec<u8>>()) {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), &data).unwrap();
        let _ = JImageReader::open(temp.path());
    }
}
