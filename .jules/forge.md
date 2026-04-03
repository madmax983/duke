**Extract God Function**
**Learning:** `bootstrap_stdlib` was over 3600 lines long, a classic God Function anti-pattern that cluttered the `lib.rs` file. Moving it to its own file dramatically improved readability.
**Action:** Always look for massive setup or registration functions and extract them to dedicated modules, using `pub(crate)` to share internal helpers.

**Extract God Test Module**
**Learning:** `crates/duke-interpreter/src/lib.rs` had an enormous 20,000+ line inline `#[cfg(test)] mod tests { ... }` block that was half the size of the file, making it a "God Object" file that's incredibly hard to navigate.
**Action:** Extract huge inline test modules into dedicated `tests.rs` files using `#[cfg(test)] mod tests;` to maintain module boundaries while improving source readability.
