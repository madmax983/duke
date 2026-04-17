1. **Fix Clippy `clippy::unnecessary_sort_by` in `duke-telemetry/src/lib.rs`**: Change `sort_by(|a, b| b...cmp(&a...))` to `sort_by_key(|b| std::cmp::Reverse(b...))`.
2. **Fix Clippy `clippy::map_unwrap_or` in `duke-interpreter/src/native.rs`**: Change `.map(...).unwrap_or(...)` to `.map_or(..., ...)`.
3. **Verify Fix**: Run `cargo clippy --all-targets --all-features -- -D warnings` to verify.
4. **Pre-commit**: Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
5. **Submit**: Create a commit with the title "⚒️ Forge: Fix clippy warnings" and a description detailing the fixes.
