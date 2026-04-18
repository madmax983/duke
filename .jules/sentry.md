## 2026-04-16 - Increased code coverage for duke-telemetry
**Learning:** Using tools like `cargo tarpaulin` allowed finding untested behavior in `duke-telemetry`. Many `assert!` clauses were checking for string content that relied upon default settings. I added several targeted tests for `duke-bytecode` and `duke-telemetry`.
**Action:** Adding new tests to handle explicit empty states and different JSON outputs helps prevent regressions in formatting logic.
## 2025-04-18 - [Switch Instruction Edge Cases]
**Learning:** Found coverage gaps in bytecode decoding for `Tableswitch` and `Lookupswitch` where negative bounds (e.g., `high < low` or `npairs < 0`) were technically handled in the code but unverified by tests.
**Action:** Always ensure negative limits and arithmetic underflows on parsed binary instruction headers are tested to ensure they cleanly fail and don't panic.
