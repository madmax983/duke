**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.
**[Encapsulate Module Facades]
**Tangle:** The `pub mod` declarations for internal sub-modules like `reachability` and `ser_helpers` were publicly exposed, violating encapsulation boundaries.
**Blueprint:** Changed the visibilities to `pub(crate) mod` to encapsulate the modules, while re-exporting only the specific items like `find_shortest_path` where needed.
**[Encapsulate `duke_classfile` Facade]**
**[Encapsulate `duke_classfile` Facade]**
**Tangle:** The `duke_classfile` API exported an intermediate `signature` module (`pub mod signature`) merely to re-export its inner types. This creates confusing, leaky abstractions and redundant paths.
**Blueprint:** Encapsulated the module. In `duke_classfile`, the `signature` module's contents are already re-exported via `pub use`, eliminating the `signature` namespace from the public API entirely by making it `pub(crate)`.
