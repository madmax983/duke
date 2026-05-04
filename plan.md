1. Add a `#[cfg(test)] mod tests` block testing the `with_atomic_*` and `load_atomic_reference` helper functions in `crates/duke-interpreter/src/native.rs`.
   - The tests will cover valid payloads as well as invalid payloads that return `Error::InvalidRef`.
2. Format the code with `cargo fmt --all`.
3. Check code for lints with `cargo clippy --all-targets --all-features -- -D warnings`.
4. Run `cargo test` to ensure tests pass.
5. Create a `sentry.md` journal entry if applicable.
6. Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
7. Submit the PR using `gh pr create` via bash command.
