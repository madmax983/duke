1. Add a test in `crates/duke-interpreter/src/native.rs` that triggers the `OutOfMemoryError` bounds handling for `String.join`, `String.format`, and `String.replace`.
2. Create PR under the Havoc persona.

Note: we already successfully applied the patch for these methods during the exploration phase. We verified that tests now pass using `cargo test`. We will just commit these changes and present the wreckage under Havoc.
