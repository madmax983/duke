**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.
**[Encapsulate Module Facades]
**Tangle:** The `pub mod` declarations for internal sub-modules like `reachability` and `ser_helpers` were publicly exposed, violating encapsulation boundaries.
**Blueprint:** Changed the visibilities to `pub(crate) mod` to encapsulate the modules, while re-exporting only the specific items like `find_shortest_path` where needed.
**[Replacing Wildcard Exports with Explicit Exports]
**Tangle:** Several crates (`duke-interpreter`, `duke-telemetry`) were using wildcard exports (`pub use module::*`) in their `lib.rs`, which leaks internal implementation details and creates unclear public API boundaries.
**Blueprint:** Replaced the wildcard exports with explicit exports (e.g., `pub use context::{ClassContext, ...};`) to enforce strict API boundaries and reduce potential coupling issues.
