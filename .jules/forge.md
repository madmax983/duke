**Extract tests to standalone file**
**Learning:** Oversized inline test modules (`mod tests { ... }`) in massive files like `lib.rs` cause bloat.
**Action:** Extract inline test modules into dedicated `tests.rs` files, keeping `use super::*;` at the top and applying `#[cfg(test)] mod tests;` in the parent file.
**Extract tests to standalone file**
**Learning:** Oversized inline test modules (`mod tests { ... }`) in massive files like `lib.rs` cause bloat.
**Action:** Extract inline test modules into dedicated `tests.rs` files, keeping `use super::*;` at the top and applying `#[cfg(test)] mod tests;` in the parent file.
