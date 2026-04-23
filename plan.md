1. We identified a critical system fragility in the `duke-interpreter` crate where multiple locks (`RwLock` for `ZIP_FILES`, `Mutex` for `SYSTEM_PROPERTY_OVERRIDES`, and `CompletionRuntime`) used `.unwrap()` blindly.
2. If any Java thread (or a bug in the VM execution layer) panicked while holding the lock, the lock was "poisoned".
3. All subsequent native method calls on those locks `.unwrap()` the PoisonError, resulting in a cascading panic that immediately brings down the entire Rust process (instead of propagating a recoverable error to the VM).
4. We simulated this vulnerability and proved it using standard `std::sync` primitives or `loom`.
5. We replaced these dangerous unwraps in `native.rs` with `.unwrap_or_else(|poisoned| poisoned.into_inner())` to gracefully recover the lock data and continue execution even if a panic occurred previously.
6. We replaced `.unwrap()` for `heap.get_mut(r)` inside value boxing methods with safe `?` operators or `if let Ok(obj) = heap.get_mut(r)`.
7. Pre-commit check to ensure proper testing, verification, review, and reflection are done.
1. **Fix missing documentation for `LambdaInfo` fields.**
   - Modify `crates/duke-interpreter/src/registry.rs` to document all fields of `LambdaInfo` to resolve `missing_docs` errors.
2. **Fix intra-doc link warnings in `duke-loader`.**
   - Modify `crates/duke-loader/src/lib.rs` to replace `[`module`]` links with `` `module` `` for private modules to avoid `rustdoc::private_intra_doc_links` warnings.
3. **Verify docs and tests.**
   - Run `RUSTFLAGS="-D missing_docs" cargo check --all-targets` and `cargo doc --no-deps` to ensure warnings are resolved. Run tests.
4. **Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.**
   - Run `pre_commit_instructions` and follow them.
5. **Submit the change.**
   - Create a PR with title "🎻 Bard: [documentation update]" detailing the chapter, insight, and examples.
