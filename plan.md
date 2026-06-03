1. **Fix String.indexOf Bounds Panic**
   - Execute a Python script to fix `native_string_indexof_from` and `native_string_last_indexof_from` in `crates/duke-interpreter/src/native.rs` to map Java `fromIndex` using `.char_indices().nth(from)` instead of using it as a raw byte index, which previously caused slicing panics on multi-byte characters.

2. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
   - Run `pre_commit_instructions` and follow the steps.

3. **Submit the PR**
   - Run `cargo test` and `cargo fmt`.
   - Submit with title "👺 Havoc: [Panic] String.indexOf and String.lastIndexOf multi-byte split".
