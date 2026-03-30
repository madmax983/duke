**[Title]
**Tangle:** [The Structural Mess]
**Blueprint:** [The Structural Fix]

**Refactoring the Duke Interpreter Blob**
**Tangle:** The `crates/duke-interpreter/src/lib.rs` file had grown into a massive "Blob" anti-pattern (over 23,000 lines). It mixed JVM data structures, the registry, execution engine logic, and an enormous amount of standard library bootstrapping/tests, making it hard to navigate and violating the single responsibility principle.
**Blueprint:** Extracted the core execution context data structures (`MethodEntry`, `FieldEntry`, `ExceptionEntry`, `ClassContext`) into a new `context` module, and the registry types (`ClassRegistry`, `NativeRegistry`, handler definitions) into a new `registry` module. These are now re-exported from `lib.rs` to maintain a clean public API while breaking up the physical file bloat.

**Refactoring the Duke GC Blob**
**Tangle:** The `crates/duke-gc/src/lib.rs` file was a God Struct, mixing garbage collection and memory allocation responsibilities with I/O subsystems, such as OS-level file handles, sockets, and subprocesses management.
**Blueprint:** Extracted the core OS-level file, process, and socket management data structures (`HostFileHandle`, `HostProcessHandle`, `SpawnedProcessIds`) and related methods out of `Heap` into a separate `HostManager` struct within a new `host` module. The `Heap` struct now delegates I/O calls to a `host: HostManager` field, enforcing high cohesion and low coupling.
