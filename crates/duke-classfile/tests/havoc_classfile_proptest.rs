//! Proptest suite for the `duke-classfile` parser.
//!
//! This module contains property-based tests that feed random byte sequences
//! into the classfile parser to ensure it fails gracefully without panicking
//! or crashing, enforcing the parser's robustness against malformed input.

use proptest::prelude::*;

proptest! {
    #[test]
    fn does_not_crash_on_random_bytes(data in any::<Vec<u8>>()) {
        let _ = duke_classfile::parse(&data);
    }
}
