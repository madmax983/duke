## 2024-06-17 - [Standardize Result Type Aliases]
**Tangle:** The four core crates (`duke-bytecode`, `duke-classfile`, `duke-loader`, `duke-runtime`) exposed generic `Result` aliases like `pub type Result<T> = std::result::Result<T, Error>;`, leading to ambiguous paths when these types were imported across the workspace.
**Blueprint:** Updated all generic `Result` aliases to explicitly refer to their module-specific error enum (e.g., `pub type Result<T> = std::result::Result<T, crate::Error>;`). This enforces a clear boundary and prevents cross-crate visibility or resolution issues.
