**Extract God Function**
**Learning:** `bootstrap_stdlib` was over 3600 lines long, a classic God Function anti-pattern that cluttered the `lib.rs` file. Moving it to its own file dramatically improved readability.
**Action:** Always look for massive setup or registration functions and extract them to dedicated modules, using `pub(crate)` to share internal helpers.

**Extract massive match blocks**
**Learning:** Massive `match` statements over opcodes in interpreters/decoders (like `decode_one` with 270 lines and >100 cases) create a severe Pyramid of Doom and God Function anti-pattern, making the code incredibly hard to read.
**Action:** Group opcodes logically (e.g., constants, loads, stores, math) and extract them into dedicated helper functions that return `Option<Result>`. The main loop can then route the opcode to these helpers, drastically flattening the structure without runtime overhead.
