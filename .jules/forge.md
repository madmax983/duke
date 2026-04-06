**Extract God Function**
**Learning:** `bootstrap_stdlib` was over 3600 lines long, a classic God Function anti-pattern that cluttered the `lib.rs` file. Moving it to its own file dramatically improved readability.
**Action:** Always look for massive setup or registration functions and extract them to dedicated modules, using `pub(crate)` to share internal helpers.

**Extract massive inline test modules**
**Learning:** Inline test modules that contain thousands of lines (e.g. over 25000 lines in `lib.rs`) act as extreme bloat that ruins readability. Moving them to dedicated `tests.rs` files dramatically improves navigability without affecting behavior.
**Action:** Constantly monitor massive files for oversized `mod tests { ... }` blocks and extract them.
