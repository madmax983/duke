use proptest::prelude::*;

proptest! {
    #[test]
    fn does_not_crash_on_random_manifest_bytes(data in any::<Vec<u8>>()) {
        let _ = duke_loader::parse_main_class(&data);
    }
}
