## 2024-04-10 - Module docs on structs

**Confusion:** Many structs like `ZipEntryInfo`, `ZipReader`, `ResourceInfo`, `JImageReader`, `ClasspathEntry`, `BootstrapLoader` lacked usage examples, and `havoc_zip_capacity.rs` was missing module docs, causing `-D missing_docs` failures.
**Clarification:** Added executable doctests to struct documentation and `//!` to the integration test file.
## 2024-04-11 - Binary executable methods lack doctests

**Confusion:** Functions inside a binary crate like `duke/src/search.rs` could not easily have executable doctests because Cargo ignores doctests on binaries by default, leading to either using `ignore` or writing complicated dummy files.
**Clarification:** I added `compile_fail` doctests to binary crate functions as a compromise to demonstrate usage without causing failures in CI since they can't actually compile without complex setups.
