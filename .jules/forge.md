**Extract God Function**
**Learning:** `bootstrap_stdlib` was over 3600 lines long, a classic God Function anti-pattern that cluttered the `lib.rs` file. Moving it to its own file dramatically improved readability.
**Action:** Always look for massive setup or registration functions and extract them to dedicated modules, using `pub(crate)` to share internal helpers.

**Extract God Object inline mod tests**
**Learning:** The `crates/duke-interpreter/src/lib.rs` file had a massive inline `#[cfg(test)] mod tests { ... }` block (over 25,000 lines long). This "Blob" anti-pattern severely bloated the file and hurt readability, making it extremely difficult to navigate the actual core logic. Extracting it to a dedicated `tests.rs` file significantly reduced bloat while preserving all original logic and tests.
**Action:** Always check for massively overgrown inline test modules in large files. When extracting, remove the outer `{}` braces from the extracted content and apply the `#[cfg(test)]` attribute directly to the new `mod tests;` declaration in the parent file to prevent compilation in release builds.
