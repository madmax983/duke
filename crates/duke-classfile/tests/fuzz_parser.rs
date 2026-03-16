use duke_classfile::parse;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 10000, .. ProptestConfig::default() })]
    #[test]
    fn test_classfile_parser_fuzz(bytes in any::<Vec<u8>>()) {
        let _ = parse(&bytes);
    }
}
