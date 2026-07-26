1. **Fix `rustdoc::private_intra_doc_links` in `duke-gc/src/lib.rs`**
   - The method `compact_old` links to `Self::mark_old`, but `mark_old` is private.
   - Using `replace_with_git_merge_diff`, downgrade the link from `[`mark_old`]: Self::mark_old` to inline backticks ` `mark_old` `. Remove the reference definition `[`mark_old`]: Self::mark_old` entirely from the doc comment block.

2. **Fix `rustdoc::private_intra_doc_links` in `duke-interpreter/src/registry.rs`**
   - The method `enable_real_jdk_shadow` links to `KEEP_SYNTHETIC`, but `KEEP_SYNTHETIC` is private.
   - Using `replace_with_git_merge_diff`, downgrade the intra-doc link `[`KEEP_SYNTHETIC`]` to inline code formatting ` `KEEP_SYNTHETIC` ` so it does not trigger the broken links warning.

3. **Fix `missing_docs` crate warnings in integration tests**
   - The `duke-classfile/tests/havoc_classfile_proptest.rs`, `duke-interpreter/tests/havoc_slice_panics.rs` and `duke/tests/havoc_jdwp_oom.rs` files are missing module-level documentation. Add `#![allow(missing_docs)]` doc comments to the top of these files to describe their purpose and fix the warning. (Like the other havoc tests)

4. **Complete pre commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

5. **Submit the PR**
   - Execute `cargo test` and `cargo doc --no-deps --all-features` to ensure no warnings or failures exist.
   - Create a PR titled '🎻 Bard: [documentation update]' following Bard's guidelines.
