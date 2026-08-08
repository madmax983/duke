use crate::helpers::ser_helpers::*;

fn test() {
    // missing lines: 16, 17, 25, 33, 41, 49, 50, 51
    // These correspond to the closures passed into keyed_map, the `let mut v = ...` and the `.serialize(ser)`.
    // Wait, the closure definition line might be marked missing if it's never executed, BUT we did execute them in `test_site3`, `test_site2_u16`, `test_pair_str`, `test_sorted_set`!
    // Why are lines 16, 17, 25, 33, 41, 49, 50, 51 missing coverage? Let's check `cargo tarpaulin` output again carefully.
}
