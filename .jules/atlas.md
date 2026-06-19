**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.
**[Encapsulate Module Facades]
**Tangle:** The `pub mod` declarations for internal sub-modules like `reachability` and `ser_helpers` were publicly exposed, violating encapsulation boundaries.
**Blueprint:** Changed the visibilities to `pub(crate) mod` to encapsulate the modules, while re-exporting only the specific items like `find_shortest_path` where needed.
**[Encapsulate `duke_classfile` Facade]**
**Tangle:** The `duke_classfile` API exported an intermediate `types` module (`pub mod types`) merely to re-export its inner types, and `duke-telemetry` exposed internal serialization helpers via `pub mod ser_helpers`. This creates confusing, leaky abstractions and redundant paths.
**Blueprint:** Encapsulated both modules. In `duke_classfile`, replaced `pub mod types` with direct `pub use` statements at the crate root, eliminating the `types` namespace from the public API entirely. In `duke_telemetry`, reduced `ser_helpers` visibility to `pub(crate)`.
**[Encapsulate `duke_telemetry` Facade]**
**Tangle:** `duke-telemetry` exposed internal serialization helpers via `mod helpers` in `lib.rs` (originally `pub(crate) mod helpers`). This leaked implementation details across modules.
**Blueprint:** Encapsulated the module. In `duke_telemetry`, reduced `helpers` visibility from `pub(crate) mod helpers` to `mod helpers`.

**[Fix `duke_classfile` Import Regression]**
**Tangle:** `duke/src/jar_diff.rs` imported types from `duke_classfile::types::*`, which caused a compilation error under the `nova` feature flag because the `types` module was previously removed from `duke_classfile`'s public API.
**Blueprint:** Updated the imports in `jar_diff.rs` to consume types directly from the `duke_classfile` root (e.g., `duke_classfile::AttributeData`).
