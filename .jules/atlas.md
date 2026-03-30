**[Title]
**Tangle:** [The Structural Mess]
**Blueprint:** [The Structural Fix]

**Refactoring the Duke Interpreter Blob**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file had grown into a massive "Blob" anti-pattern (over 23,000 lines). It mixed JVM data structures, the registry, execution engine logic, and an enormous amount of standard library bootstrapping/tests, making it hard to navigate and violating the single responsibility principle.
**Blueprint:** Extracted the core execution context data structures (`MethodEntry`, `FieldEntry`, `ExceptionEntry`, `ClassContext`) into a new `context` module, and the registry types (`ClassRegistry`, `NativeRegistry`, handler definitions) into a new `registry` module. These are now re-exported from `lib.rs` to maintain a clean public API while breaking up the physical file bloat.
**Extracting Execution Engine from Interpreter Blob]
**Tangle:** The file `crates/duke-interpreter/src/lib.rs` had grown to over 27,000 lines, tightly coupling the core bytecode execution engine with hundreds of native Java method implementations and standard library bootstrapping.
**Blueprint:** Extracted the core execution loop (`execute`, `execute_class`), related context-building logic, and reflection helpers into a new module `crates/duke-interpreter/src/engine.rs`. Exposed necessary internal helpers via `pub(crate)` and utilized `pub use engine::*;` in `lib.rs` to maintain the public API contract while drastically reducing the file size and separating concerns.
