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
