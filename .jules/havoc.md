## 2025-02-28 - `.unwrap()` panics in concurrent operations
**Learning:** Found `.unwrap()` calls in `native.rs` mapping `Option` and `Result` values, such as capacity and regex limits, which can be triggered maliciously to crash the JVM process.
**Action:** Replaced these unsafe assertions with safe fallbacks such as `.unwrap_or(0)` or mapping error defaults to keep the VM resilient.
