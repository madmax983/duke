**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.
**[Encapsulate Module Facades]
**Tangle:** The `pub mod` declarations for internal sub-modules like `reachability` and `ser_helpers` were publicly exposed, violating encapsulation boundaries.
**Blueprint:** Changed the visibilities to `pub(crate) mod` to encapsulate the modules, while re-exporting only the specific items like `find_shortest_path` where needed.
**[Encapsulate `duke_classfile` Facade]**
**Tangle:** The `duke_classfile` API exported an intermediate `types` module (`pub mod types`) merely to re-export its inner types, and `duke-telemetry` exposed internal serialization helpers via `pub mod ser_helpers`. This creates confusing, leaky abstractions and redundant paths.
**Blueprint:** Encapsulated both modules. In `duke_classfile`, replaced `pub mod types` with direct `pub use` statements at the crate root, eliminating the `types` namespace from the public API entirely. In `duke_telemetry`, reduced `ser_helpers` visibility to `pub(crate)`.
**[Fix Clippy large_stack_frames in execution.rs]**
**Tangle:** The `run_execution` function inside `crates/duke-interpreter/src/execution.rs` is inherently massive and allocates ~550KB on the stack, exceeding the 512KB Clippy limit. Attempting to box local structs inside the function after `.cloned()` is ineffective and creates a heap allocation penalty because the struct must still be cloned on the stack first.
**Blueprint:** Acknowledged the function's structural scale and applied the `#[allow(clippy::large_stack_frames)]` lint directly to `run_execution` to clear the warning without introducing a performance penalty.
