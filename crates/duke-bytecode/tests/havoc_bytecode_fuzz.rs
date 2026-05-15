#![allow(missing_docs)]

use duke_bytecode::decode;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]
    #[test]
    fn fuzz_bytecode_decode(data in prop::collection::vec(any::<u8>(), 0..1024)) {
        let _ = decode(&data);
    }
}
