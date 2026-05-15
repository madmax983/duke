#![allow(missing_docs)]

use duke_classfile::parse;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]
    #[test]
    fn fuzz_classfile_parse(data in prop::collection::vec(any::<u8>(), 0..1024)) {
        let _ = parse(&data);
    }
}
