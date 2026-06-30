**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.
**[Encapsulate Module Facades]
**Tangle:** The `pub mod` declarations for internal sub-modules like `reachability` and `ser_helpers` were publicly exposed, violating encapsulation boundaries.
**Blueprint:** Changed the visibilities to `pub(crate) mod` to encapsulate the modules, while re-exporting only the specific items like `find_shortest_path` where needed.
**[Encapsulate `duke_classfile` Facade]**
**Tangle:** The `duke_classfile` API exported an intermediate `types` module (`pub mod types`) merely to re-export its inner types, and `duke-telemetry` exposed internal serialization helpers via `pub mod ser_helpers`. This creates confusing, leaky abstractions and redundant paths.
**Blueprint:** Encapsulated both modules. In `duke_classfile`, replaced `pub mod types` with direct `pub use` statements at the crate root, eliminating the `types` namespace from the public API entirely. In `duke_telemetry`, reduced `ser_helpers` visibility to `pub(crate)`.

**Extract AtomicPayload and related structures into a separate module**
**Tangle:** The `duke-gc/src/lib.rs` file had grown significantly (over 2500 lines) and contained unrelated domain responsibilities. Specifically, the definition of `AtomicPayload` and its various related synthetic concurrency states (e.g., `ReentrantLockState`, `ConditionState`, `ExecutorState`, `CountDownLatchState`, `SemaphoreState`, `CyclicBarrierState`) were heavily inflating the core garbage collector implementation. This represents a "Blob" anti-pattern, tangling GC logic with host-side concurrency abstractions.
**Blueprint:** Extracted the `AtomicPayload` enum, its implementations, and all associated synthetic state structures into a new `atomic.rs` module. The new module boundary isolates the host-side concurrency representations, leaving `duke-gc/src/lib.rs` more focused on generational memory management (`Heap`, `HeapObject`, GC cycles). Exposed necessary traits and structures via `pub use atomic::*;` in the crate root to preserve the public API and maintain compatibility.
