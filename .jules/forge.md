**Extract God Function**
**Learning:** `bootstrap_stdlib` was over 3600 lines long, a classic God Function anti-pattern that cluttered the `lib.rs` file. Moving it to its own file dramatically improved readability.
**Action:** Always look for massive setup or registration functions and extract them to dedicated modules, using `pub(crate)` to share internal helpers.
**Extract inline tests in `duke-interpreter` to `tests.rs`**
**Learning:** `crates/duke-interpreter/src/lib.rs` had grown to >50,000 lines, mostly due to a giant inline `mod tests { ... }`.
**Action:** Extract the test module into `tests.rs` and use `#[cfg(test)] mod tests;` to maintain clean separation. This shrinks the file by over 25,000 lines, drastically improving readability.
