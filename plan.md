1. **Identify the Weak Point**: `native_string_split` and `native_string_split_limit` in `duke-interpreter/src/native.rs` both use `.unwrap()` on `regex::Regex::new`.
2. **Prove Fragility**: We wrote `test_string_split_limit_panic_trigger` which triggered a panic when given a regex exceeding the compilation size limit.
3. **Fix and Validate**: We modified the code to propagate errors via `regex_pattern_syntax_error`, catching the error rather than panicking.
4. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
5. **Submit PR**.
