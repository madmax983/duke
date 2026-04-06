**Extract God Function**
**Learning:** `bootstrap_stdlib` was over 3600 lines long, a classic God Function anti-pattern that cluttered the `lib.rs` file. Moving it to its own file dramatically improved readability.
**Action:** Always look for massive setup or registration functions and extract them to dedicated modules, using `pub(crate)` to share internal helpers.
**Extract inline tests from massive files**
**Learning:** Inline test modules in very large files (e.g., >50k lines) severely bloat the file and increase cognitive load.
**Action:** Always extract massive `mod tests` blocks into dedicated `tests.rs` files, replacing the inline block with `#[cfg(test)] mod tests;`. Ensure `use super::*;` is at the top of the new file.
