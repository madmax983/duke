## 2024-05-18 - [Missing root-level classfile types]
**Confusion:** The workspace builds correctly with `cargo build`, but fails to document with `cargo doc` due to `duke_classfile::types::{...}` not being correctly resolved even though the types are re-exported at the crate root.
**Clarification:** Remove the `types::` prefix and pull directly from the `duke_classfile` crate root.
