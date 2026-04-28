1. **Explore `crates/duke-loader/src/zip.rs` to identify nesting and logic that can be extracted.**
   - Target functions that handle complex logic like parsing and file extraction.
   - Refactor `parse_central_directory` to use cleaner iterator chains or extract helper functions for extracting individual fields.
   - Target repetitive error propagation patterns that can be flattened.
2. **Review `crates/duke-loader/src/jimage.rs` for large functions.**
   - Focus on `build_index` which has a deeply nested loop reading attributes.
   - Extract the attribute reading loop into a helper function (e.g., `parse_location_attributes`).
3. **Verify refactor correctness using `cargo test` and `cargo clippy`.**
   - Since these are strict refactorings, tests should still pass.
4. **Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.**
5. **Submit a PR with the required title and structure.**
