1. Add a test file `crates/duke-telemetry/tests/print_and_markdown.rs` to test the empty cases of `print_bytecode_cost` and `markdown_bytecode_cost`.
2. Add a `#[test]` module at the bottom of `crates/duke-telemetry/src/helpers.rs` to cover the empty paths or missing serialization paths. Wait, the coverage report says `helpers.rs` has missing lines 15, 16, 24, 32, 40, 48, 49, 50. Let's check those lines.
3. Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
4. Submit the changes.
