## 2024-04-10 - Module docs on structs

**Confusion:** Many structs like `ZipEntryInfo`, `ZipReader`, `ResourceInfo`, `JImageReader`, `ClasspathEntry`, `BootstrapLoader` lacked usage examples, and `havoc_zip_capacity.rs` was missing module docs, causing `-D missing_docs` failures.
**Clarification:** Added executable doctests to struct documentation and `//!` to the integration test file.
## 2024-04-10 - Executable Doctests vs Empty Blocks
**Confusion:** I initially added a ````no_run` block with just comments instead of an actual executable example for `run_execution`, and I used a broken import path for `ClassFile` in `deps_graph.rs`. This caused `cargo test` to fail on the doctests.
**Clarification:** Doctests must be fully executable Rust code, even if we need to mock or initialize large structs. Imports in doctests must exactly match the public API of the crate being tested.
