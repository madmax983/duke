**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.
**[Encapsulate Module Facades]
**Tangle:** The `pub mod` declarations for internal sub-modules like `reachability` and `ser_helpers` were publicly exposed, violating encapsulation boundaries.
**Blueprint:** Changed the visibilities to `pub(crate) mod` to encapsulate the modules, while re-exporting only the specific items like `find_shortest_path` where needed.
**[Encapsulate `duke_classfile` Facade]**
**Tangle:** The `duke_classfile` API exported an intermediate `types` module (`pub mod types`) merely to re-export its inner types, and `duke-telemetry` exposed internal serialization helpers via `pub mod ser_helpers`. This creates confusing, leaky abstractions and redundant paths.
**Blueprint:** Encapsulated both modules. In `duke_classfile`, replaced `pub mod types` with direct `pub use` statements at the crate root, eliminating the `types` namespace from the public API entirely. In `duke_telemetry`, reduced `ser_helpers` visibility to `pub(crate)`.
**[The Blob of included natives]**
**Tangle:** `duke-interpreter/src/lib.rs` uses `include!` for 13 native implementation files. Attempting to extract these into a standard `pub(crate) mod native;` breaks internal scoping for test macros like `wrap_simple_native_for_tests!`, which implicitly expect the native functions to reside directly in the crate root.
**Blueprint:** Acknowledged the structural complexity and risk of test breakage. Decided not to refactor the `include!` pattern at this time, as a complete architectural fix requires a deep rewrite of the test macro paths, which violates the 'Atlas avoids: Moving large chunks of code that will break 50+ import paths' guideline without prior explicit intent. Applied a scoped fix for the `clippy::large_stack_frames` on `run_execution` instead to maintain the pipeline.
