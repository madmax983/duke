## 2024-04-22 - [LambdaInfo Documentation]
**Confusion:** The purpose of `LambdaInfo` and its fields were completely undocumented, making it difficult to understand how dynamic lambda generation works.
**Clarification:** Added module-level documentation and executable doctests explaining how `LambdaInfo` bridges the gap between functional interfaces and implementation methods during `invokedynamic` instructions.

## 2024-05-18 - [ClassContext Documentation]
**Confusion:** The purpose and usage of `build_class_context` in bridging raw classfile data and execution environments was entirely undocumented, leading to confusion about its initialization requirements.
**Clarification:** Added comprehensive documentation, explaining that it serves as the runtime representation of a loaded class. Also added an executable doctest demonstrating the precise constant pool initialization needed for successful parsing.

## 2024-05-19 - [Missing Telemetry Test Docs]
**Confusion:** The integration test file for telemetry print formatting triggered missing documentation warnings, which are explicitly mandated by strict linting rules.
**Clarification:** Added module-level `//!` documentation to the `print_and_markdown.rs` integration test, explaining its purpose in verifying report text output structure.

## 2024-05-20 - [SwitchTargets Documentation]
**Confusion:** The `SwitchTargets` iterator for `tableswitch` and `lookupswitch` was completely undocumented, making it difficult to understand how switch targets are resolved at runtime.
**Clarification:** Added module-level documentation and an executable doctest explaining how `SwitchTargets` unifies the two switch mechanisms and provides a clean iterator over match values and offsets.

## 2024-05-21 - [ReflectedAnnotationConst Documentation]
**Confusion:** The `ReflectedAnnotationConst` enum lacked documentation for its variants and was missing module-level examples, leading to `missing_docs` clippy warnings and making it hard to understand how raw JVM annotation values map to Rust.
**Clarification:** Added complete documentation for each enum variant, explaining their JVM equivalents, and included an executable doctest demonstrating basic usage.

## 2024-05-22 - [Bytecode Instruction/Types Documentation]
**Confusion:** The massive `Instruction` enum in `duke-bytecode` and related types like `ArrayType` were entirely undocumented, causing the `missing_docs` clippy lint to be explicitly suppressed via `#[allow(missing_docs)]`. The same issue was present with intra-doc links in `duke-classfile/src/error.rs` causing warning when running `cargo doc`.
**Clarification:** Added comprehensive module-level and struct-level documentation for `Instruction` and `ArrayType`, along with executable doctests demonstrating their usage and structural layout. Fixed broken intra-doc link in `duke-classfile/src/error.rs`.

## 2024-05-23 - [Test Files Missing Docs]
**Confusion:** Proptest and loom test binaries generated missing documentation warnings, which shouldn't require full crate documentation blocks.
**Clarification:** Suppressed warnings in `havoc_jimage_proptest.rs`, `havoc_zip_proptest.rs`, and `havoc_zip_files_loom.rs` using `#![allow(missing_docs)]`. Added `//!` crate documentation to `duke-interpreter` to clarify its core orchestration role.

## 2024-05-19 - Documenting Structs in pub(crate) modules
**Confusion:** `rustdoc` tests fail to find structs when you try to import them using their private path (`use duke_loader::zip::ZipReader`), and changing `pub(crate)` to `pub` breaks module encapsulation.
**Clarification:** If a struct in a `pub(crate)` module is already exported publicly at the crate root via `pub use zip::ZipReader`, import it directly from the crate root in your doc tests (`use duke_loader::ZipReader;`). This allows executable examples without polluting the public API.
## 2024-05-24 - [Test Files Missing Docs]
**Confusion:** The `oss_jar_smoke.rs` integration test generated missing documentation warnings.
**Clarification:** Added `//!` crate documentation to `oss_jar_smoke.rs`.

## 2024-05-24 - [Instruction Enum Documentation]
**Confusion:** The massive `Instruction` enum lacked documentation for its variants and contained a global `#[allow(missing_docs)]` suppression, creating a gap in understanding JVM opcodes.
**Clarification:** Removed the suppression and documented all ~200 variants. Learned that while narrative documentation is usually preferred, for massive, standardized enums like opcodes, concise descriptions (e.g., `/// Push null.`) are better for maintainability.
## 2024-05-25 - [Broken Intra-Doc Links]
**Confusion:** Resolving intra-doc links to private items or across crates without explicit paths triggers warnings, but merely removing links decreases usability.
**Clarification:** Downgraded private intra-doc links to standard backticks (e.g. `[`item`]` to `` `item` ``). Fully qualified cross-crate links (e.g. `duke_gc::Heap::set_string_layout`) to ensure proper resolution while maintaining clickability.
