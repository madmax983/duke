use duke_loader::manifest;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_manifest(data in any::<Vec<u8>>()) {
        let _ = manifest::parse_main_class(&data);
    }
}
