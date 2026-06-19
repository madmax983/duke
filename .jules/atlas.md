**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.
**[Encapsulate Module Facades]
**Tangle:** The `pub mod` declarations for internal sub-modules like `reachability` and `ser_helpers` were publicly exposed, violating encapsulation boundaries.
**Blueprint:** Changed the visibilities to `pub(crate) mod` to encapsulate the modules, while re-exporting only the specific items like `find_shortest_path` where needed.
**[Encapsulate `duke_classfile` Facade]**
**Tangle:** The `duke_classfile` API exported an intermediate `types` module (`pub mod types`) merely to re-export its inner types, and `duke-telemetry` exposed internal serialization helpers via `pub mod ser_helpers`. This creates confusing, leaky abstractions and redundant paths.
**Blueprint:** Encapsulated both modules. In `duke_classfile`, replaced `pub mod types` with direct `pub use` statements at the crate root, eliminating the `types` namespace from the public API entirely. In `duke_telemetry`, reduced `ser_helpers` visibility to `pub(crate)`.
**Encapsulate Telemetry Facade**
**Tangle:** The `duke_telemetry` crate exported a `pub mod ser_helpers` exposing internal serialization helper functions to the workspace, violating encapsulation boundaries and leaking internal implementation details.
**Blueprint:** Modified the visibility of `ser_helpers` to `pub(crate) mod ser_helpers` in `crates/duke-telemetry/src/helpers.rs`, successfully encapsulating the module within the crate.

**Fix type annotations and undefined imports in duke binary**
**Tangle:** `duke` binary was failing to compile due to missing type annotations for closure parameters when calling `.and_then()` on `Option` reference, and importing `types::CpEntry` from `duke-classfile` where `types` was removed.
**Blueprint:** Added explicit type annotations `&Option<CpEntry>` to the closure parameters, and updated the import to remove `types::` module namespace.

**Standardize module encapsulation**
**Tangle:** The `pub(crate) mod` declarations for submodules in all workspace crates exposed unnecessary access flags since `mod` is private by default.
**Blueprint:** Modified `pub(crate) mod` to `mod` across `duke-bytecode`, `duke-classfile`, `duke-gc`, `duke-interpreter`, `duke-loader`, `duke-runtime`, and `duke-telemetry`. This improves encapsulation and reduces redundant visibility specifiers while correctly restricting module internals and re-exporting only what's needed.
