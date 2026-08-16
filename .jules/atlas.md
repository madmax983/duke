**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.
**[Encapsulate Module Facades]
**Tangle:** The `pub mod` declarations for internal sub-modules like `reachability` and `ser_helpers` were publicly exposed, violating encapsulation boundaries.
**Blueprint:** Changed the visibilities to `pub(crate) mod` to encapsulate the modules, while re-exporting only the specific items like `find_shortest_path` where needed.
**[Encapsulate `duke_classfile` and `duke_telemetry` Facades]**
**Tangle:** The `duke_classfile` API exported an intermediate `signature` module (`pub mod signature`), and `duke-telemetry` exposed internal serialization helpers via `pub mod ser_helpers`. This creates confusing, leaky abstractions and redundant paths since their types/functions are re-exported.
**Blueprint:** Encapsulated both modules. In `duke_classfile`, changed `signature` to `pub(crate) mod signature;`, eliminating the namespace from the public API since its types are re-exported at the root. In `duke_telemetry`, reduced `ser_helpers` visibility to `pub(crate) mod ser_helpers`.
