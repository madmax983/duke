**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.

**Restrict Internal Submodules**
**Tangle:** The `reachability` module in `duke-bytecode` was exposed as `pub mod` but acted strictly as an internal implementation detail, leaking the architectural boundary.
**Blueprint:** Altered its visibility to `pub(crate) mod` and explicitly exported only the necessary types in the facade `lib.rs` (via `pub use`) to enforce encapsulation.
