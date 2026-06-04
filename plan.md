1. **Refactor `if let Ok` to `let-else` in `crates/duke-interpreter/src/native.rs`**
   - Stage the changes made to `crates/duke-interpreter/src/native.rs`.
   - Use `run_in_bash_session` to execute `git add crates/duke-interpreter/src/native.rs && rm fix_box.py`.
2. **Refactor `if let Ok` to `let-else` in `crates/duke-interpreter/src/stdlib.rs`**
   - Stage the changes made to `crates/duke-interpreter/src/stdlib.rs`.
   - Use `run_in_bash_session` to execute `git add crates/duke-interpreter/src/stdlib.rs && rm fix_stdlib.py`.
3. **Run tests to verify**
   - Run `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, and `cargo fmt --all` using `run_in_bash_session`.
4. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
5. **Commit the branch**
   - Use `run_in_bash_session` to execute `echo -e "⚒️ Forge: [let-else refactor]\n\n🚮 Smell: Deeply nested 'if let Ok' blocks acting as pyramids of doom in native.rs and stdlib.rs.\n✨ Solution: Flattened 'if let Ok' into 'let Ok(...) = ... else { return }' guard clauses.\n🧼 Benefit: Reduces cognitive load, limits indentation, and improves readability.\n🛡️ Verification: Tests passed. No logic changed." > pr_desc.txt && git commit -F pr_desc.txt && rm pr_desc.txt`.
