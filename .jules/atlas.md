**[Title]
**Tangle:** [The Structural Mess]
**Blueprint:** [The Structural Fix]

**Refactoring the Duke Interpreter Blob**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file had grown into a massive "Blob" anti-pattern (over 23,000 lines). It mixed JVM data structures, the registry, execution engine logic, and an enormous amount of standard library bootstrapping/tests, making it hard to navigate and violating the single responsibility principle.
**Blueprint:** Extracted the core execution context data structures (`MethodEntry`, `FieldEntry`, `ExceptionEntry`, `ClassContext`) into a new `context` module, and the registry types (`ClassRegistry`, `NativeRegistry`, handler definitions) into a new `registry` module. These are now re-exported from `lib.rs` to maintain a clean public API while breaking up the physical file bloat.

**Unified Error Handling Types**
**Tangle:** Each crate (`duke-bytecode`, `duke-classfile`, `duke-loader`, `duke-runtime`) had its own named error type (`DecodeError`, `VerifyError`, `ParseError`, `LoadError`, `VmError`) and result type. This led to an inconsistent API surface and "Trait Pollution" across the workspace when handling cross-crate failures.
**Blueprint:** Standardized error types across modules by renaming crate-specific errors to `crate::Error` and `crate::Result<T>` using `thiserror`. Combined `DecodeError` and `VerifyError` into a centralized `duke_bytecode::Error` enum. Aliased the old names to maintain backward compatibility and avoid breaking the public API ("The Facade").

**Extracting Native module and Tests**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file was a 38,000 line "Blob", coupling the interpreter engine, massive amounts of native methods implementations, and all execution tests.
**Blueprint:** Extracting the native module into `crates/duke-interpreter/src/native.rs` and the execution tests into a separate `tests` module is architecturally correct. However, doing so requires making ~40 internal helper functions and structures `pub` or `pub(crate)` across `lib.rs` and `native.rs` due to severe circular dependencies in the tests. A better architectural fix requires refactoring the tests to use a public facade or builder pattern instead of directly accessing interpreter internals, which is out of scope for a single safe refactor. The code is left unchanged to avoid breaking "The Public API" rule with excessive `pub` leakages.
