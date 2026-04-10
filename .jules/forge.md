**Extract God Function**
**Learning:** `bootstrap_stdlib` was over 3600 lines long, a classic God Function anti-pattern that cluttered the `lib.rs` file. Moving it to its own file dramatically improved readability.
**Action:** Always look for massive setup or registration functions and extract them to dedicated modules, using `pub(crate)` to share internal helpers.

**Extract Pyramid of Doom**
**Learning:** `parse_constant_pool` in `crates/duke-classfile/src/parser.rs` contained a massive, deeply nested `match` block for parsing constant pool entries. This "Pyramid of Doom" made the function difficult to read and maintain. Extracting the `match` block into a separate helper function, `parse_cp_entry`, flattened the structure and significantly improved clarity without altering the logic.
**Action:** Always identify deeply nested logic, especially large `match` or `if/else` statements, and extract them into named helper functions to flatten the code and reduce cognitive load.
