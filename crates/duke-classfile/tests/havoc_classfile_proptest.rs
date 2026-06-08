//! Proptests for the `duke_classfile` parser.
//!
//! This test suite subjects the parser to randomized byte inputs to ensure it never panics.
//! It helps identify edge cases or unstructured data that might otherwise cause the JVM to crash.

use proptest::prelude::*;

proptest! {
    #[test]
    fn does_not_crash_on_random_bytes(data in any::<Vec<u8>>()) {
        let _ = duke_classfile::parse(&data);
    }
}
