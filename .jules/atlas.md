**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.
**[Encapsulate Module Facades]
**Tangle:** The `pub mod` declarations for internal sub-modules like `reachability` and `ser_helpers` were publicly exposed, violating encapsulation boundaries.
**Blueprint:** Changed the visibilities to `pub(crate) mod` to encapsulate the modules, while re-exporting only the specific items like `find_shortest_path` where needed.
**[Encapsulate `duke_classfile` Facade]**
**Tangle:** The `duke_classfile` API exported an intermediate `types` module (`pub mod types`) merely to re-export its inner types, and `duke-telemetry` exposed internal serialization helpers via `pub mod ser_helpers`. This creates confusing, leaky abstractions and redundant paths.
**Blueprint:** Encapsulated both modules. In `duke_classfile`, replaced `pub mod types` with direct `pub use` statements at the crate root, eliminating the `types` namespace from the public API entirely. In `duke_telemetry`, reduced `ser_helpers` visibility to `pub(crate)`.
**[Fix Module Path and Type Inference]**
**Tangle:** `duke/src/jar_diff.rs` attempted to import items from a non-existent `types` module (`duke_classfile::types::{AttributeData, CpEntry, CpIndex}`) and failed type inference for `Option::as_ref`.
**Blueprint:** Updated imports to use the crate root directly (`duke_classfile::{AttributeData, CpEntry, CpIndex}`) as those types are re-exported. Explicitly annotated closures with `&Option<CpEntry>` to resolve compiler type inference errors.
