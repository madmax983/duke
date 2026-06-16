1. Remove temporary Python scripts created during exploration (`patch_cfg.py`, `patch_cfg_fix.py`, `patch_native_string.py`, `patch_native_string2.py`, `test_native_string.py`) using `rm` to clean up the workspace.
2. Verify the changes applied to `crates/duke-bytecode/src/cfg.rs` during exploration using `git diff`.
3. Run `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test` to ensure the codebase format is correct and tests pass without any warnings.
4. Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
5. Create a new journal entry in `.jules/bolt.md` if any critical learnings are made, otherwise proceed to submit the change.
6. Submit PR with the title "⚡ Bolt: Replace HashMap lookup with binary_search in CFG generation" and detailed description.
