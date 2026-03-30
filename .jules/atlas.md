**[Title]
**Tangle:** [The Structural Mess]
**Blueprint:** [The Structural Fix]

**Refactoring the Duke Interpreter Blob**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file had grown into a massive "Blob" anti-pattern (over 23,000 lines). It mixed JVM data structures, the registry, execution engine logic, and an enormous amount of standard library bootstrapping/tests, making it hard to navigate and violating the single responsibility principle.
**Blueprint:** Extracted the core execution context data structures (`MethodEntry`, `FieldEntry`, `ExceptionEntry`, `ClassContext`) into a new `context` module, and the registry types (`ClassRegistry`, `NativeRegistry`, handler definitions) into a new `registry` module. These are now re-exported from `lib.rs` to maintain a clean public API while breaking up the physical file bloat.

**Refactoring the Duke Interpreter Blob (Native Methods)**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file grew to over 26,000 lines due to the inline definitions of more than a hundred standard library native method implementations.
**Blueprint:** Extracted all `fn native_*` implementations and their associated helper functions into a new `natives.rs` module. Declared the module as `pub(crate) mod natives;` in `lib.rs` to enforce proper encapsulation and updated all internal references and test wrappers to use the `natives::` path.
