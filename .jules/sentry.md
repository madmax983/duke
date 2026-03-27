## 2026-03-24 - Tarpaulin output interpretation
**Learning:** Lcov traces output by `cargo-tarpaulin` explicitly label covered lines as `DA:<line_number>,1` (or higher) and explicitly label uncovered lines as `DA:<line_number>,0`. It is easy to misinterpret grep results if not looking strictly for the `,0` suffix.
**Action:** When parsing `lcov.info` for coverage gaps, always grep specifically for `DA:.*,0` and double-check the surrounding context lines to ensure the line is truly uncovered before planning to write a test. Always delete `lcov.info` before committing to avoid polluting the repo.
## 2024-05-24 - clippy::unreadable_literal
**Learning:** Large numeric literals, especially hex strings used in testing, can trigger `clippy::unreadable_literal` under `-D warnings`.
**Action:** Use underscores in large numeric literals (e.g., `0x1234_5678`) to maintain readability and avoid CI failures.
