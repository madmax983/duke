## 2024-04-10 - Module docs on structs

**Confusion:** Many structs like `ZipEntryInfo`, `ZipReader`, `ResourceInfo`, `JImageReader`, `ClasspathEntry`, `BootstrapLoader` lacked usage examples, and `havoc_zip_capacity.rs` was missing module docs, causing `-D missing_docs` failures.
**Clarification:** Added executable doctests to struct documentation and `//!` to the integration test file.
## 2024-04-11 - Binary executable methods lack doctests

**Confusion:** Functions inside a binary crate like `duke/src/search.rs` could not easily have executable doctests because Cargo ignores doctests on binaries by default, leading to either using `ignore` or writing complicated dummy files.
**Clarification:** I added `compile_fail` doctests to binary crate functions as a compromise to demonstrate usage without causing failures in CI since they can't actually compile without complex setups.
## 2024-04-11 - Doc tests should be complete compiling examples
**Confusion:** I used `compile_fail` blocks to show pseudo code and avoid compiler errors when missing setup code. Code reviewer correctly noted that we must use full working code in Examples, not broken code.
**Clarification:** Rewrote doc-test code snippets in `execution.rs` and `native.rs` to mock necessary elements (e.g., `ExecutionState::default()`) rather than rely on `compile_fail` with pseudo-code.
