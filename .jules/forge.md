**Extract God Function**
**Learning:** `bootstrap_stdlib` was over 3600 lines long, a classic God Function anti-pattern that cluttered the `lib.rs` file. Moving it to its own file dramatically improved readability.
**Action:** Always look for massive setup or registration functions and extract them to dedicated modules, using `pub(crate)` to share internal helpers.

**Extract God Test Module**
**Learning:** Inline `#[cfg(test)] mod tests { ... }` blocks in massive files (like the 58k-line `duke-interpreter/src/lib.rs`) destroy navigability and create "God Files".
**Action:** Always extract test modules containing thousands of lines into a dedicated `tests.rs` file and include them with `#[cfg(test)] mod tests;`, making sure to strip the wrapper to avoid module inception.
