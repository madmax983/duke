**[Title]
**Tangle:** [The Structural Mess]
**Blueprint:** [The Structural Fix]

**Refactoring the Duke Interpreter Blob**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file had grown into a massive "Blob" anti-pattern (over 23,000 lines). It mixed JVM data structures, the registry, execution engine logic, and an enormous amount of standard library bootstrapping/tests, making it hard to navigate and violating the single responsibility principle.
**Blueprint:** Extracted the core execution context data structures (`MethodEntry`, `FieldEntry`, `ExceptionEntry`, `ClassContext`) into a new `context` module, and the registry types (`ClassRegistry`, `NativeRegistry`, handler definitions) into a new `registry` module. These are now re-exported from `lib.rs` to maintain a clean public API while breaking up the physical file bloat.

**Unified Error Handling Types**
**Tangle:** Each crate (`duke-bytecode`, `duke-classfile`, `duke-loader`, `duke-runtime`) had its own named error type (`DecodeError`, `VerifyError`, `ParseError`, `LoadError`, `VmError`) and result type. This led to an inconsistent API surface and "Trait Pollution" across the workspace when handling cross-crate failures.
**Blueprint:** Standardized error types across modules by renaming crate-specific errors to `crate::Error` and `crate::Result<T>` using `thiserror`. Combined `DecodeError` and `VerifyError` into a centralized `duke_bytecode::Error` enum. Aliased the old names to maintain backward compatibility and avoid breaking the public API ("The Facade").
**[Split ClassFile types]
**Tangle:** `duke-classfile/src/types.rs` was a Blob anti-pattern holding constant pool, attributes, and class structures.
**Blueprint:** Split into `constant_pool.rs`, `attributes.rs`, and `class.rs` to match domain responsibilities.
**[Extract Host IO from GC Heap]
**Tangle:** The `duke-gc/src/lib.rs` file mixed OS host file and process logic with GC logic within the `Heap` struct, creating a "God Struct" and a 2,500-line Blob.
**Blueprint:** Extracted host IO into a dedicated `HostManager` struct in `host.rs`. Updated interpreter to call `heap.host.open_host_zip()`.
