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
