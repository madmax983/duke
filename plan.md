1. Write a harness for `ZIP` parsing (`havoc_zip_oom_crash.rs`) to trigger OOM by crafting an archive where `METHOD_STORED` or `METHOD_DEFLATED` is used and `compressed_size` is near `usize::MAX`.
2. Apply a fix to `duke-loader::zip::read_entry_info` to limit `compressed_size` to a sane value.
3. Write a harness for `JImage` parsing (`havoc_jimage_oom_crash.rs`) to trigger OOM by setting `info.uncompressed` to a very large value.
4. Apply a similar fix to `duke-loader::jimage::read_resource` limiting `raw_len` to a sane value.
5. Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
6. Submit a PR titled "👺 Havoc: Zip & JImage Memory Bomb".
