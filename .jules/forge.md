**Extract God Function**
**Learning:** `bootstrap_stdlib` was over 3600 lines long, a classic God Function anti-pattern that cluttered the `lib.rs` file. Moving it to its own file dramatically improved readability.
**Action:** Always look for massive setup or registration functions and extract them to dedicated modules, using `pub(crate)` to share internal helpers.
**Extract run_execution**
**Learning:** `run_execution` was over 3300 lines long, a God Function anti-pattern that cluttered the `lib.rs` file. Moving it to its own file dramatically improved readability.
**Action:** Always look for massive execution/dispatch loops and extract them to dedicated modules, using `pub(crate)` to share internal helpers.

**Extract Native Functions**
**Learning:** The JVM `native_*` handlers occupied over 25,000 lines in `lib.rs`, acting as a God Object that overwhelmed the file. Grouping them inside their own included module `native.rs` massively clarifies the codebase.
**Action:** Always look for massive contiguous blocks of conceptually similar handlers or bindings and extract them to dedicated modules, using `include!("native.rs")` or `pub(crate)` modules to safely share internal items without nesting hell.

**Extract Common Boilerplate**
**Learning:** Repeated extraction of arguments and fields via unwrapping `unwrap_or(Slot::Reference(None))` cluttered `native.rs` and added unnecessary cognitive load. Creating inline helpers (`extract_slot_arg`, `extract_field_arg`) simplified hundreds of call sites.
**Action:** Identify repeated primitive boilerplate and condense it into named helper functions.

**Refactoring Repeated Primitive Extraction Boilerplate**
**Learning:** Found 22+ instances of `args.get(idx).copied().unwrap_or(Slot::...)` throughout `native.rs` and `stdlib.rs`. This kind of repeated primitive extraction boilerplate bloats the code, reduces readability, and increases cognitive load, masking the actual business logic of the VM natives.
**Action:** Created a `SlotExt` trait in `duke_runtime` to encapsulate these extractions into small, explicitly named helper methods (`unwrap_or_ref()`, `unwrap_or_int()`, etc.). Applied this refactoring to simplify and flatten the structure of the native implementations, enforcing DRY principles without changing runtime behavior.
