**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.
**[Encapsulate Module Facades]
**Tangle:** The `pub mod` declarations for internal sub-modules like `reachability` and `ser_helpers` were publicly exposed, violating encapsulation boundaries.
**Blueprint:** Changed the visibilities to `pub(crate) mod` to encapsulate the modules, while re-exporting only the specific items like `find_shortest_path` where needed.
**[Encapsulate `duke_classfile` Facade]**
**Tangle:** The `duke_classfile` API exported an intermediate `types` module (`pub mod types`) merely to re-export its inner types, and `duke-telemetry` exposed internal serialization helpers via `pub mod ser_helpers`. This creates confusing, leaky abstractions and redundant paths.
**Blueprint:** Encapsulated both modules. In `duke_classfile`, replaced `pub mod types` with direct `pub use` statements at the crate root, eliminating the `types` namespace from the public API entirely. In `duke_telemetry`, reduced `ser_helpers` visibility to `pub(crate)`.
**[Fix Leaky Abstraction in Module Facade]**
**Tangle:** The `duke` crate's `jar_diff.rs` was still importing an intermediate `types` module from `duke_classfile` (i.e. `use duke_classfile::types::{...}`), which leaked after the previous encapsulation of `duke_classfile` that removed `pub mod types`. This caused a compilation error.
**Blueprint:** Removed the `types::` prefix from the import path in `duke::jar_diff.rs` to correctly use the flat exports directly from the `duke_classfile` root (e.g. `use duke_classfile::{parse, AttributeData, CpEntry, CpIndex}`).
