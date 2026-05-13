//! Fuzzing and property tests for the `duke-classfile` parser.
//!
//! This module contains tests using `proptest` to ensure the parser does not panic on arbitrary or malformed input.

use proptest::prelude::*;

proptest! {
    #[test]
    fn does_not_crash_on_random_bytes(data in any::<Vec<u8>>()) {
        let _ = duke_classfile::parse(&data);
    }
}
