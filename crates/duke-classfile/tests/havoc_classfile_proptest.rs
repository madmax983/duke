//! Fuzz testing for the classfile parser to ensure it does not panic on arbitrary input.

use proptest::prelude::*;

proptest! {
    #[test]
    fn does_not_crash_on_random_bytes(data in any::<Vec<u8>>()) {
        let _ = duke_classfile::parse(&data);
    }
}
