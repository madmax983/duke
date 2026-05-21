1. Use `run_in_bash_session` to execute a Python script (`unify_errors.py`) that implements the "Encapsulate Module Facades" architectural change by:
   - Modifying `crates/duke-telemetry/src/helpers.rs` to replace `pub mod ser_helpers` with `pub(crate) mod ser_helpers`, properly encapsulating the internal serialization test helpers.
   - Deleting the python script after execution.
2. Use `run_in_bash_session` to run `cargo check` to verify the automated refactoring succeeded.
3. Use `run_in_bash_session` to run `cargo test --workspace` to verify the codebase behaves correctly with the new visibility.
4. Use `run_in_bash_session` to append the CRITICAL learning regarding "Encapsulate Module Facades" to `.jules/atlas.md` in the required `**Tangle:** / **Blueprint:**` format via a bash heredoc.
5. Use `run_in_bash_session` to write the verbatim multi-line commit message to `commit_msg.txt` and PR description to `pr_body.txt` via a bash heredoc.
6. Use `run_in_bash_session` to run the full workspace test suite `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, and `cargo fmt --all`.
7. Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
8. Use `run_in_bash_session` to execute explicit `git add -A`, `git commit -F commit_msg.txt`, and `git push` commands to submit the branch `atlas_error` using the generated PR text.
