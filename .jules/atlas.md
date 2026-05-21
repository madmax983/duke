**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.
**[Encapsulate Module Facades]
**Tangle:** The `pub mod` declarations for internal sub-modules like `reachability` and `ser_helpers` were publicly exposed, violating encapsulation boundaries.
**Blueprint:** Changed the visibilities to `pub(crate) mod` to encapsulate the modules, while re-exporting only the specific items like `find_shortest_path` where needed.
**[Encapsulate `duke_classfile` Facade]**
**Tangle:** The `duke_classfile` API exported an intermediate `types` module (`pub mod types`) merely to re-export its inner types, and `duke-telemetry` exposed internal serialization helpers via `pub mod ser_helpers`. This creates confusing, leaky abstractions and redundant paths.
**Blueprint:** Encapsulated both modules. In `duke_classfile`, replaced `pub mod types` with direct `pub use` statements at the crate root, eliminating the `types` namespace from the public API entirely. In `duke_telemetry`, reduced `ser_helpers` visibility to `pub(crate)`.
**[Encapsulate Module Facades]**
**Tangle:** The `ser_helpers` module in `duke-telemetry` was exported as `pub mod ser_helpers`, incorrectly leaking test helpers into the public API of the crate and violating strict visibility boundaries.
**Blueprint:** Modified `pub mod ser_helpers` to `pub(crate) mod ser_helpers`, correctly encapsulating the module within the crate.
