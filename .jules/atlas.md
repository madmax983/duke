**[Title]
**Tangle:** [The Structural Mess]
**Blueprint:** [The Structural Fix]

**Refactoring the Duke Interpreter Blob**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file had grown into a massive "Blob" anti-pattern (over 23,000 lines). It mixed JVM data structures, the registry, execution engine logic, and an enormous amount of standard library bootstrapping/tests, making it hard to navigate and violating the single responsibility principle.
**Blueprint:** Extracted the core execution context data structures (`MethodEntry`, `FieldEntry`, `ExceptionEntry`, `ClassContext`) into a new `context` module, and the registry types (`ClassRegistry`, `NativeRegistry`, handler definitions) into a new `registry` module. These are now re-exported from `lib.rs` to maintain a clean public API while breaking up the physical file bloat.

**Extracting Standard Library Natives from Interpreter**
**Tangle:** `crates/duke-interpreter/src/lib.rs` was over 25,000 lines long, filled with `native_*` standard library implementations and `bootstrap_stdlib` logic, making the main execution file an unmanageable Blob.
**Blueprint:** Extracted the standard library bootstrapping (`bootstrap_stdlib`) and all its associated `native_*` handlers into a new `crates/duke-interpreter/src/stdlib.rs` module. Exposed required shared internal functions (e.g., `format_java_double`, `stringify_slot`, `heap_object_to_string`) by upgrading their visibility to `pub(crate)` so both modules can share them cleanly. Fixed import boundaries in tests.
