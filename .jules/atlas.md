**Extract tests module into separate file**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file was ~38,000 lines long, containing ~20,000 lines of tests in an inline `mod tests { ... }`.
**Blueprint:** Extracted the `tests` module into `crates/duke-interpreter/src/tests.rs` and replaced the inline block with `#[cfg(test)] mod tests;` to enforce better separation of concerns and reduce file bloat.
