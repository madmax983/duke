**[Title]
**Tangle:** [The Structural Mess]
**Blueprint:** [The Structural Fix]

**Refactoring the Duke Interpreter Blob**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file had grown into a massive "Blob" anti-pattern (over 23,000 lines). It mixed JVM data structures, the registry, execution engine logic, and an enormous amount of standard library bootstrapping/tests, making it hard to navigate and violating the single responsibility principle.
**Blueprint:** Extracted the core execution context data structures (`MethodEntry`, `FieldEntry`, `ExceptionEntry`, `ClassContext`) into a new `context` module, and the registry types (`ClassRegistry`, `NativeRegistry`, handler definitions) into a new `registry` module. These are now re-exported from `lib.rs` to maintain a clean public API while breaking up the physical file bloat.

**Unified Error Handling Types**
**Tangle:** Each crate (`duke-bytecode`, `duke-classfile`, `duke-loader`, `duke-runtime`) had its own named error type (`DecodeError`, `VerifyError`, `ParseError`, `LoadError`, `VmError`) and result type. This led to an inconsistent API surface and "Trait Pollution" across the workspace when handling cross-crate failures.
**Blueprint:** Standardized error types across modules by renaming crate-specific errors to `crate::Error` and `crate::Result<T>` using `thiserror`. Combined `DecodeError` and `VerifyError` into a centralized `duke_bytecode::Error` enum. Aliased the old names to maintain backward compatibility and avoid breaking the public API ("The Facade").

**Extracting Unit Tests from lib.rs Blob**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file was suffering from the "Blob" anti-pattern, growing to over 38,000 lines. The native implementations and the primary execution loop are heavily intertwined (e.g., GC `gather_roots` and `patch_forwarded_slots`), causing severe cyclic dependency and visibility ("Leaky Abstraction") issues when attempting to extract native handlers into a separate module.
**Blueprint:** Instead of forcing an unnatural boundary that leaks private execution state, the massive `#[cfg(test)] mod tests` block (comprising over 20,000 lines of test functions, mocks, and helper macros) was extracted into a separate `crates/duke-interpreter/src/tests.rs` file. This cleanly cuts the `lib.rs` blob in half without exposing internal JVM engine state, retaining high cohesion for the execution loop while drastically improving file readability.
