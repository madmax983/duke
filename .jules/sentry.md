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
