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
