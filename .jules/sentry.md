## 2024-05-24 - Testing Format and Serialization Output Gaps
**Learning:** Functions that generate user-facing outputs or interact with deep serialization layers like `serde` might fail testing because error flows only occur when standard types wrap custom objects specifically built to fail during serialization. Additionally, testing format outputs (like empty reports vs populated reports) requires full mock states or testing for length boundaries inside iterator consumers like `take(10)`.
**Action:** When filling telemetry or reporting coverage gaps, use custom struct mock objects (e.g. `FailingSerializer`) or explicitly trigger less-than bounds logic (e.g. 1 item for a `take(10)`) to get 100% path coverage for outputs.
## 2026-04-16 - Increased code coverage for duke-telemetry
**Learning:** Using tools like `cargo tarpaulin` allowed finding untested behavior in `duke-telemetry`. Many `assert!` clauses were checking for string content that relied upon default settings. I added several targeted tests for `duke-bytecode` and `duke-telemetry`.
**Action:** Adding new tests to handle explicit empty states and different JSON outputs helps prevent regressions in formatting logic.
## 2024-04-18 - Tested Default Implementations of CallbackOps
**Learning:** Default implementations for traits (like `CallbackOps`) are easy to overlook when searching for missing test coverage, but they still contain logic (such as returning specific `VmError` variants) that should be validated.
**Action:** Always check the trait definitions themselves for default methods, and write a mock struct to execute and verify those defaults.
## 2024-04-18 - Verified unreachable branches and invalid handle error conditions
**Learning:** Found code that implements traits or default configurations lacking tests. Unreachable blocks inside exhaustive matching are occasionally completely untestable but we can cover the valid combinations to show behavior correctly processes options.
**Action:** Adding precise tests for invalid I/O handle edge cases (e.g. `accept_connection` on a non-listener fd) boosts confidence that malicious or corrupted state correctly traps into safe errors instead of causing logic bugs.
## 2024-05-20 - Testing serialization helpers
**Learning:** Found an uncovered file `helpers.rs` within `duke-telemetry`. It was mostly formatting maps via `serde::Serialize` wrapper structs. These paths were only reachable when outputting telemetry JSON, which had very shallow testing coverage.
**Action:** In addition to adding specific structural tests per helper wrapper (`site3`, `site2_u16`, `pair_str`, `sorted_set`), defining minimal dummy structs deriving `Serialize` inside the `#[cfg(test)] mod tests` block allowed for targeted snapshotting of `serde_json::to_string` to easily verify output structure without constructing giant overarching Telemetry logs.
## 2024-04-20 - Testing CFG HTML and Bytecode Cost

**Learning:** It can be hard to reach coverage for visual representation helpers (like html rendering for a class) because sometimes they depend on specific bytecodes being present, or having enough fields to match strings properly. Also, simple errors (like io errors) are easy to reach in testing by creating explicit fake classes, or using small code snippets with specific constraints (like testing `test_decoder_invalid_tableswitch_high_less_than_low` instead of `test_decoder_tableswitch_too_many_entries`).
**Action:** When increasing coverage for simple visualizer components, use isolated, fake class definitions with specific interfaces and fields to verify html layout. When doing bytecode decoding, craft small custom byte buffers to simulate exact errors instead of large, complex byte buffers.
## 2024-04-24 - Formatting Unwraps in Reports
**Learning:** Using a custom struct that implements `Write` and intentionally returns `io::Error` is a clean way to test the error paths of display or report functions, making sure that errors propagate or get handled instead of just running the happy path.
**Action:** Implement a `FailingWriter` dummy object for testing formatting and printing APIs.
## 2024-05-24 - Testing JImage Parsing Bounds
**Learning:** JImage parsing edge cases with artificially corrupted files (`0x0001_0000` version tag required) that use `u64::MAX` or out-of-bounds offset metadata effectively verify robust error propagation in `duke_loader::JImageReader::read_resource`, ensuring we catch regressions that could otherwise cause out-of-bounds panics or incorrect bounds checks against raw uncompressed lengths.
**Action:** Always test metadata parsers using edge-case inputs (e.g. `u64::MAX`, bounds mismatches) to verify format-specific bounds checking fails gracefully rather than panicking on indexing.
## 2024-05-24 - Testing Zip Reader Truncated Central Directories
**Learning:** Certain edge cases in `parse_central_directory` related to `checked_add` bounds checking (like `name_start + filename_len > cd_end`) were previously uncovered, because most fuzzed zips generate completely malformed data rather than specific boundary truncations.
**Action:** Craft specific byte buffers representing minimal zip central directories, manually defining exact `filename_len` or `cd_size` values that force the `checked_add` validation to fail gracefully without overflowing.

## 2024-05-24 - Testing Duke Telemetry Serde Map Emptiness
**Learning:** `keyed_map` formatting utilities using `serialize_map` were missing tests to ensure empty HashMaps correctly serialize as `{}`.
**Action:** Adding tests for `HashMap::new()` in serialization wrappers proves correct behavior.
## 2024-05-24 - Testing Duke Internal Registry Paths
**Learning:** Found an uncovered error case in `duke-interpreter` registry related to checking `ensure_loaded_with_code_source`.
**Action:** Adding tests to test the edge case when the fallback path is used or the path is missing.

## 2024-05-24 - Atomic Types Testing
**Learning:** Added test coverage for `duke-interpreter` internal atomic extraction mechanisms `with_atomic_i32`, `with_atomic_bool` and `with_atomic_i64`. When these methods are queried with an invalid or unexpected atomic object type, they must successfully propagate an error.
**Action:** Tested the fallback error conditions, avoiding runtime crashes due to mistyped values being placed into the `AtomicPayload` fields.

## 2024-05-24 - Path Handling Edge Cases
**Learning:** Found uncovered path resolution boundary checks in `duke-loader`'s `DirectoryLoader::resolve_child_path`.
**Action:** Asserted that potentially unsafe file paths (such as `..`, `/absolute`, `\\backslash` or `C:colon`) correctly throw safe missing file IO errors instead of enabling directory traversal risks.
## 2026-05-02 - Testing JDWP Command Dispatching
**Learning:** The Java Debug Wire Protocol (JDWP) command dispatcher in `duke/src/jdwp.rs` involves complex state tracking with atomic variables to manage the debugged VM's state (suspended or running).
**Action:** Created isolated unit tests utilizing mock atomics to accurately verify the suspend/resume dispatch logic, string parsing, and binary payload formatting responses without requiring full network socket integration tests.
## 2024-05-24 - Testing ReentrantReadWriteLock Native Operations
**Learning:** Found several native functions implementing `ReentrantReadWriteLock` and `PriorityQueue` operations missing test coverage in `duke-interpreter`. Adding dummy tests correctly executes these logic branches without needing fully mocked multi-threading scenarios.
**Action:** Add inline unit tests using a mocked heap and manually setting up the `obj_ref` payload.

## 2024-05-20 - Unchecked Writer Failures
**Learning:** `std::io::Write` methods can fail at any point (e.g. disk full, broken pipe), and large formatting routines often ignore intermediate `Result::Err` returns from formatting macros, risking swallowed errors or silent panics.
**Action:** Write tests that pass a mock `std::io::Write` which deterministically fails after N bytes to verify `Err` propagation at all intermediate points of the formatting logic.
