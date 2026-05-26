1. Modify `crates/duke-telemetry/src/helpers.rs` to encapsulate the `ser_helpers` module by changing `pub mod ser_helpers` to `pub(crate) mod ser_helpers`.
2. Modify `crates/duke-classfile/src/lib.rs` to remove the redundant `types` module abstraction if present. Based on exploration, `duke_classfile` already exports types directly. Ensure all usages of `duke_classfile::types` in `duke/src/jar_diff.rs` are removed. Use `sed` to fix the usage in `duke/src/jar_diff.rs`.
3. Verify the changes using `run_in_bash_session` to execute `cargo check --all-targets --all-features`.
4. Verify the changes visually using `git diff`.
5. Run tests using `cargo test --all-targets --all-features`.
6. Run `cargo clippy --all-targets --all-features -- -D warnings`.
7. Run `cargo fmt --all`.
8. Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
9. Write the PR description to `msg.txt` using `echo -e "🗺️ Atlas: [architectural change]\n\n**Tangle:** The \`duke_classfile\` API exposed an intermediate \`types\` module redundantly, and \`duke-telemetry\` leaked internal serialization helpers via \`pub mod ser_helpers\`.\n**Blueprint:** Encapsulated \`ser_helpers\` in \`duke-telemetry\` using \`pub(crate)\` and removed redundant \`types\` path from \`duke_classfile\` usages in \`duke/src/jar_diff.rs\`.\n**Stability:** Reduced coupling, faster compile times and clearer encapsulation.\n**Verification:** Builds successfully, strict separation enforced." > msg.txt`
10. Commit changes using `git add . && git commit -F msg.txt`.
