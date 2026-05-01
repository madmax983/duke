1. Add fuzz tests and property tests to verify system stability and edge cases.
    - Write a proptest for `DirectoryLoader::find_class` to check for path traversal vulnerabilities in `crates/duke-loader/tests/havoc_directory_loader_fuzz.rs`.
    - Write a proptest for `JImageReader::open` in `crates/duke-loader/tests/havoc_jimage_fuzz.rs` to fuzz the jimage parsing format.
    - Add tests for `duke_bytecode` to verify we don't OOM on corrupted `lookupswitch`/`tableswitch` tables in `crates/duke-bytecode/tests/havoc_bytecode_oom.rs`.
    - Add tests for `duke_classfile` to verify we don't OOM on massive constant pool counts, interfaces, or method counts in `crates/duke-classfile/tests/havoc_classfile_oom.rs`.
    - Add test for `duke_gc` to verify `Heap::allocate` gracefully handles bounds rather than hitting process capacity limits in `crates/duke-gc/tests/havoc_heap_oom.rs`.
    - Add test for `duke_runtime::Frame` to verify it gracefully handles large variable capacity bounds in `crates/duke-runtime/tests/havoc_frame_oom.rs`.
    - Add test for `duke_loader::ZipReader` to verify it handles fake massive EOCD entries gracefully in `crates/duke-loader/tests/havoc_zip_oom.rs`.
    - Add test for `duke_loader::JImageReader` to verify it handles corrupted header counts gracefully in `crates/duke-loader/tests/havoc_jimage_oom.rs`.
    - Add Loom test `havoc_system_properties_loom.rs` to verify thread-safe atomicity of setting system properties.
2. Complete pre-commit steps to make sure proper testing, verifications, reviews, and reflections are done.
3. Submit the change.
1. **Target**: Fuzz tests triggered a panic inside `write_field` in `duke-gc/src/lib.rs` (due to out-of-bounds indexing of `fields[field_idx] = value`).
2. **Additional Issues**:
   - `heap.young.get_mut(usize::try_from(r).unwrap())` can panic in `get_mut` and `get` if `r` does not fit in `usize`.
3. **Fix Strategy**:
   - Add a `VmError` variant with a custom message since `Error::VmError(String)` already exists. Or better, `Error::InvalidRef` already exists!
   - In `duke-runtime/src/error.rs`, add a new error variant `FieldOutOfBounds` that takes `index` and `length` to properly surface field indexing errors instead of panicking.
   - For `write_field`: Replace `obj.fields[field_idx] = value;` with safe indexing and return the new `FieldOutOfBounds` error.
   - Update `get` and `get_mut` to use `.unwrap_or(usize::MAX)` to prevent panics during conversion from `u64` to `usize`, and correctly return `Error::InvalidRef`.
1. **Understand the problem:** The problem requires fixing missing documentation warnings that `cargo clippy --all-targets --all-features -- -W missing-docs` reports.
2. **Current state:** We had missing docs for test binaries `crates/duke-loader/tests/havoc_jimage_proptest.rs`, `crates/duke-loader/tests/havoc_zip_proptest.rs`, `crates/duke-interpreter/tests/havoc_zip_files_loom.rs`.
3. **Execution:** We already fixed this by adding `#![allow(missing_docs)]` to these test files, which makes sense for test binaries that don't need crate-level documentation to satisfy the warning.
4. **Fixing other module:** We had an issue with `duke-interpreter` missing the crate documentation block. I updated `crates/duke-interpreter/src/lib.rs` to have a nice `//!` description that documents the crate-level module.
5. **Validation:** `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` completes successfully, and `cargo clippy --all-targets --all-features -- -W missing_docs` doesn't produce missing_docs warnings, and tests pass.
6. **Pre-commit step**: Execute tests and run pre commit step.
7. **Submit**: Create PR.
