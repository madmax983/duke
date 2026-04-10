use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_classfile_parser(data in any::<Vec<u8>>()) {
        let _ = duke_classfile::parse(&data);
    }
}
