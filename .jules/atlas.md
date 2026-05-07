**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.
**[Encapsulate Module Facades]
**Tangle:** The `pub mod` declarations for internal sub-modules like `reachability` and `ser_helpers` were publicly exposed, violating encapsulation boundaries.
**Blueprint:** Changed the visibilities to `pub(crate) mod` to encapsulate the modules, while re-exporting only the specific items like `find_shortest_path` where needed.
**[Explicit Exports over Wildcards]**
**Tangle:** The `lib.rs` files across modules like `duke-telemetry`, `duke-interpreter`, and `duke-classfile` used wildcard exports (`pub use module::*`). This created leaky abstractions, potentially exposing internal implementation details and cluttering the module's public interface, which violated clear domain boundary principles.
**Blueprint:** Replaced all `pub use module::*` statements with exact item exports (`pub use module::{SpecificType1, SpecificType2};`), enforcing strict and intentional public APIs that only expose what is necessary.
