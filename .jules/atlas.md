**Standardize Workspace Error Types**
**Tangle:** Each module defined its own duplicate aliases like `VmResult`, `LoadResult`, and `ParseError`, breaking standardized `Result<T, crate::Error>` rules.
**Blueprint:** Removed domain-specific error aliases in favor of standard `Result` and `Error` types across all crates.

**[Unified CFG Target Extraction]**
**Tangle:** Logic for extracting control flow targets was duplicated across `cfg`, `basic_block`, and `reachability`, causing shotgun surgery.
**Blueprint:** Unified target extraction into a central `Instruction::control_flow_targets` method to enforce strict cohesion and single source of truth.
