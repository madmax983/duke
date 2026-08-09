1. **Explore target files for refactoring**
   - Check `duke/src/jar_analyze.rs` and `duke/src/audit.rs` for deeply nested `if let Ok(...)` and `if let AttributeData::Code(code)` blocks.
2. **Refactor Pyramid of Doom in JAR tools**
   - Apply guard clauses (`let Ok(...) = ... else { continue; }`) to flatten loops in `duke/src/jar_analyze.rs` and `duke/src/audit.rs` (and possibly others if they are prominent).
   - Ensure behavior remains exactly the same.
3. **Run tests and linters**
   - Execute `cargo fmt`, `cargo clippy`, and `cargo test` to verify changes.
4. **Complete pre commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
5. **Submit PR**
   - Submit the PR as "Forge" using `request_code_review`.
