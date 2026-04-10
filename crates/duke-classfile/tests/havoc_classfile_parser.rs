use duke_classfile::parser;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_parser_magic(data in any::<Vec<u8>>()) {
        let mut prefix = vec![0xCA, 0xFE, 0xBA, 0xBE, 0x00, 0x00, 0x00, 0x41]; // magic + java 21
        // Try to set constant pool count to 0xFFFF and see if it OOMs or hangs
        prefix.push(0xFF);
        prefix.push(0xFF);
        prefix.extend(data);
        let _ = parser::parse(&prefix);
    }
}
