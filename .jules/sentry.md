## 2026-03-24 - Tarpaulin output interpretation
**Learning:** Lcov traces output by `cargo-tarpaulin` explicitly label covered lines as `DA:<line_number>,1` (or higher) and explicitly label uncovered lines as `DA:<line_number>,0`. It is easy to misinterpret grep results if not looking strictly for the `,0` suffix.
**Action:** When parsing `lcov.info` for coverage gaps, always grep specifically for `DA:.*,0` and double-check the surrounding context lines to ensure the line is truly uncovered before planning to write a test. Always delete `lcov.info` before committing to avoid polluting the repo.
## 2024-05-18 - [Missing Coverage in `duke-gc`]
**Learning:** Found multiple uncovered edge cases and error paths in `duke-gc` related to `read_host_file_byte`, `write_host_file_byte`, `spawn_host_process`, `open_host_zip`, `bind_server_socket`, and `accept_connection`. When testing `GarbageCollector`, the struct is actually named `Heap` locally in `crates/duke-gc/src/lib.rs` and aliased to `GarbageCollector` later/externally.
**Action:** Always check the struct definitions and imports inside the file to ensure the tests compile, and use the internal type name when writing unit tests in `mod tests`.
## 2025-02-28 - [Hardcoded paths in tests cause massive coverage gaps]
**Learning:** A hardcoded Windows path in `jdk_modules_path()` caused several `duke-loader` integration tests to be skipped silently on Linux CI, dropping coverage for `JImageReader` by ~45%.
**Action:** When tests skip or coverage drops on file I/O operations, check for hardcoded OS-specific paths and replace them with dynamic resolution (like `JAVA_HOME`).
## 2024-04-15 - Uncovered Code in duke-loader and duke-bytecode

**Learning:** `Instruction::Dload` and `Instruction::IincW` bounds checking in `duke-bytecode/src/verifier.rs` were missing explicit test coverage. Similarly, inner `ZipReader` read errors (like `ZipFormat` errors when extracting files inside a nested `BOOT-INF/lib` jar) bubbling up to `find_class` callers were untested in `duke-loader/src/zip.rs`.

**Action:** Add direct unit tests covering the `check_locals` function with `Dload` and `IincW` to assure the branch bounds behave properly, and write a targeted `zip_loader_try_nested_read_entry_error` test that triggers a `ZipFormat` error in an inner jar during class finding, asserting that it correctly bubbles rather than being swallowed as a NotFound or panicking.
## 2024-05-24 - Duke Telemetry Missing Coverage
**Learning:** `helpers.rs` inside `duke-telemetry` was found to have 0% coverage. Found it using `cargo llvm-cov report` followed by a localized `grep` on `lcov.info`.
**Action:** Wrote unit tests specifically targeting all helpers: `site3`, `site2_u16`, `pair_str`, and `sorted_set` ensuring they successfully serialize HashMap keys using formatting as intended, when `feature = "telemetry"` is enabled.
