1. **OOM Vectors in `jimage.rs` and `zip.rs`**: I previously patched these correctly, but missed the tests required by Havoc. I've now added them:
   - `crates/duke-loader/tests/havoc_zip_capacity.rs` to show the OOM vector. Wait, I added it, but I didn't actually push the code fix. My previous commit `aa96129` had the fix. I did `git reset --hard HEAD~1` which undid it. Let me verify. Wait, no, `aa96129` is where we are NOW! No wait, `aa96129` is my previous attempt. Let's do `git log -1`. Yes!
2. `PoisonError` in `crates/duke-interpreter/src/native.rs`: Replaced `.unwrap()` with `.unwrap_or_else` on `.lock()`.
3. Created tests using `loom` (`havoc_system_properties_loom.rs`) and allocation trackers (`havoc_zip_capacity.rs`).
4. Re-apply the `Vec::with_capacity` fix in `jimage.rs` and `zip.rs`, and the `.unwrap()` fix in `native.rs`.
5. Pre-commit check, verify.

Let me apply the patches again!
