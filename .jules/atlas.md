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

**Extracting Host I/O from duke-gc Blob**
**Tangle:** The `duke-gc` crate's `lib.rs` file had become a massive Blob anti-pattern (over 2500 lines). It combined complex generational garbage collection logic (Eden space, copying collector, mark-sweep) with low-level host OS abstractions like file descriptors, sockets, and child process management.
**Blueprint:** Extracted the host I/O management structures (`HostFileHandle`, `HostProcessHandle`, `SpawnedProcessIds`) and all related native host methods on `Heap` into a separate `host.rs` module. The JVM host interactions are now neatly encapsulated without polluting the garbage collection logic.
**[Refactoring the Duke Telemetry Blob]
**Tangle:** The `crates/duke-telemetry/src/lib.rs` file was a massive Blob anti-pattern (over 1000 lines). It combined 6 distinct telemetry channels (bytecode cost, object lineage, class initialization, exception flow, dispatch resolution, and native boundary) along with the central store and formatting tools.
**Blueprint:** Extracted the 6 individual channels and the serde formatting helpers into distinct submodules (`bytecode_cost.rs`, `object_lineage.rs`, etc.). `lib.rs` was refactored into a clean facade that re-exports the individual channels and acts as the singular `TelemetryStore` entrypoint, maintaining backwards compatibility while restoring domain responsibility separation.
**Decouple GC from Class Loader (Zip Format)**
**Tangle:** The `duke-gc` (Garbage Collector) crate had a dependency on `duke-loader` and its `ZipReader` just to expose zip reading host handles. This violated separation of concerns, giving the memory manager direct knowledge of a specific archive format and creating sideways dependencies across core crates.
**Blueprint:** Removed `duke-loader` from `duke-gc`'s `Cargo.toml`. Dropped `ZipArchive` from `HostFileHandle` and removed all zip-related functions (`open_host_zip`, `zip_read_entry`, etc.) from `duke-gc`'s `Heap`. Recreated the zip-file state mapping in `duke-interpreter/src/native.rs` using `std::sync::OnceLock` and a static `RwLock<HashMap<i32, ZipReader>>`. Now, `duke-gc` handles pure primitive OS handles, while `duke-interpreter` safely maintains the higher-level format readers.
**Refactoring duke-bytecode Modules**
**Tangle:** The `duke-bytecode` crate had `pub mod` for all its modules (`call_graph`, `cfg`, `decoder`, `error`, `instruction`, `opcodes`, `verifier`). This exposed internal details and made the public API surface area larger than it needed to be.
**Blueprint:** Applied the "Facade" pattern by changing these modules to `pub(crate) mod` and using `pub use` to selectively re-export only the necessary items in `lib.rs`. Also updated some unused public constants in `opcodes.rs` to `pub(crate)` and added `#[allow(dead_code)]` to prevent compiler warnings.
**[Facade Refactoring]
**Tangle:** Public modules (`pub mod`) were leaking inner structures and `test` dependencies unnecessarily through direct module paths, leading to leaky abstractions.
**Blueprint:** Replaced `pub mod` with `pub(crate) mod` across workspace crates (like `duke_runtime`, `duke_loader`, `duke_telemetry`, `duke_gc`, `duke_interpreter`) to encapsulate implementation details. This forced tests and integration points to use the carefully curated `pub use` exports in the respective crate roots, strengthening the crate facade boundaries and enforcing domain boundaries without changing runtime behavior.
