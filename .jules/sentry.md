## 2026-04-16 - Increased code coverage for duke-telemetry
**Learning:** Using tools like `cargo tarpaulin` allowed finding untested behavior in `duke-telemetry`. Many `assert!` clauses were checking for string content that relied upon default settings. I added several targeted tests for `duke-bytecode` and `duke-telemetry`.
**Action:** Adding new tests to handle explicit empty states and different JSON outputs helps prevent regressions in formatting logic.
## 2024-04-18 - Tested Default Implementations of CallbackOps
**Learning:** Default implementations for traits (like `CallbackOps`) are easy to overlook when searching for missing test coverage, but they still contain logic (such as returning specific `VmError` variants) that should be validated.
**Action:** Always check the trait definitions themselves for default methods, and write a mock struct to execute and verify those defaults.
