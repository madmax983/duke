1. **Verify `decoder.rs` changes**:
   Run `git diff crates/duke-bytecode/src/decoder.rs` to verify that `unreachable!()` panic points have been properly replaced with explicit `DecodeError::UnknownOpcode` error returns and the `pc` field has been added to sub-decoder arguments.

2. **Add a robust test case in `decoder.rs`**:
   Execute a Python script using `run_in_bash_session` to append a new test case `havoc_tests` to the end of `crates/duke-bytecode/src/decoder.rs`. The test will intentionally pass an invalid opcode to `decode()` to ensure the `DecodeError::UnknownOpcode` is safely handled rather than hitting an `unreachable!()` panic. This step was already completed during exploration, but will ensure it exists.

3. **Complete workspace verification**:
   Run `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, and `cargo fmt --all` to ensure no warnings or test failures occur due to the changes.

4. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**

5. **Commit and create PR**:
   Use `run_in_bash_session` to commit the code and formulate a PR description. Use a bash heredoc to create a `commit_msg.txt` file and run `git commit -F commit_msg.txt`. The PR will be formatted with the title '🛡️ Sentry: [test coverage improvement]' along with the detailed Target, Risk, Strategy, and Verification sections.
