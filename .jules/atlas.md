**[Title]
**Tangle:** [The Structural Mess]
**Blueprint:** [The Structural Fix]

**Refactoring the Duke Interpreter Blob**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file had grown into a massive "Blob" anti-pattern (over 23,000 lines). It mixed JVM data structures, the registry, execution engine logic, and an enormous amount of standard library bootstrapping/tests, making it hard to navigate and violating the single responsibility principle.
**Blueprint:** Extracted the core execution context data structures (`MethodEntry`, `FieldEntry`, `ExceptionEntry`, `ClassContext`) into a new `context` module, and the registry types (`ClassRegistry`, `NativeRegistry`, handler definitions) into a new `registry` module. These are now re-exported from `lib.rs` to maintain a clean public API while breaking up the physical file bloat.

**Unified Error Handling Types**
**Tangle:** Each crate (`duke-bytecode`, `duke-classfile`, `duke-loader`, `duke-runtime`) had its own named error type (`DecodeError`, `VerifyError`, `ParseError`, `LoadError`, `VmError`) and result type. This led to an inconsistent API surface and "Trait Pollution" across the workspace when handling cross-crate failures.
**Blueprint:** Standardized error types across modules by renaming crate-specific errors to `crate::Error` and `crate::Result<T>` using `thiserror`. Combined `DecodeError` and `VerifyError` into a centralized `duke_bytecode::Error` enum. Aliased the old names to maintain backward compatibility and avoid breaking the public API ("The Facade").
**Extracting bootstrap_stdlib from the Interpreter Blob**
**Tangle:** `crates/duke-interpreter/src/lib.rs` had become a massive 38,000+ line "Blob" anti-pattern. While `context.rs` and `registry.rs` were extracted previously, `lib.rs` still contained thousands of lines dedicated to bootstrapping the standard library (`bootstrap_stdlib`) and defining its `native_*` method implementations, tangling core interpreter execution logic with standard library stubbing.
**Blueprint:** Extracted the 3,600+ line `bootstrap_stdlib` function into a new `crates/duke-interpreter/src/stdlib.rs` module. To avoid cyclic dependencies between the core execution engine (which relies on native helpers) and the native implementations, the `native_*` functions and helpers like `extract_ref_arg` were left in `lib.rs` but their visibility was changed to `pub(crate)` so they could be registered by `stdlib.rs`.
