**Extract God Function**
**Learning:** `bootstrap_stdlib` was over 3600 lines long, a classic God Function anti-pattern that cluttered the `lib.rs` file. Moving it to its own file dramatically improved readability.
**Action:** Always look for massive setup or registration functions and extract them to dedicated modules, using `pub(crate)` to share internal helpers.
**Extract run_execution**
**Learning:** `run_execution` was over 3300 lines long, a God Function anti-pattern that cluttered the `lib.rs` file. Moving it to its own file dramatically improved readability.
**Action:** Always look for massive execution/dispatch loops and extract them to dedicated modules, using `pub(crate)` to share internal helpers.

**Extract Native Functions**
**Learning:** The JVM `native_*` handlers occupied over 25,000 lines in `lib.rs`, acting as a God Object that overwhelmed the file. Grouping them inside their own included module `native.rs` massively clarifies the codebase.
**Action:** Always look for massive contiguous blocks of conceptually similar handlers or bindings and extract them to dedicated modules, using `include!("native.rs")` or `pub(crate)` modules to safely share internal items without nesting hell.

**Extract Common Boilerplate**
**Learning:** Repeated extraction of arguments and fields via unwrapping `unwrap_or(Slot::Reference(None))` cluttered `native.rs` and added unnecessary cognitive load. Creating inline helpers (`extract_slot_arg`, `extract_field_arg`) simplified hundreds of call sites.
**Action:** Identify repeated primitive boilerplate and condense it into named helper functions.

**Extract God Module (tests)**
**Learning:** The inline `mod tests` block in `crates/duke-interpreter/src/lib.rs` was over 29,600 lines long, which completely buried the actual logic of the library module and drastically reduced readability.
**Action:** Always extract enormous `mod tests` blocks into their own separate `tests.rs` files, linking them via `#[cfg(test)] mod tests;` to keep the parent file small, focused, and maintainable.
**[Extracted Attribute Parsers]
**Learning:** `decode_known_attribute` in `crates/duke-classfile/src/parser.rs` contained a massively deep `match` statement allocating and populating logic for every attribute type. This "God Function" approach breaks readability and cognitive boundaries.
**Action:** Always extract the internal logic of large `match` arms into strictly-typed helper functions (e.g. `decode_line_number_table`) to flatten code and keep functions short.

## 2026-04-11 - Simplify argument extraction in native handlers
**Learning:** `native.rs` had dozens of repetitions of `match args.get(1) { Some(Slot::Reference(Some(r))) => *r, _ => return Err(VmError::NullPointerException) }`. This is an unidiomatic "Pyramid of Doom" disguised as inline pattern matching, creating unnecessary clutter and masking the underlying intent of extracting a reference argument.
**Action:** Created and used `extract_ref_arg(args, 1)?` to compress 4 lines of matching boilerplate into a single line with `?` error propagation.
**[Extracted Constant Pool Parser]
**Learning:** The `parse_constant_pool` function in `crates/duke-classfile/src/parser.rs` was almost 100 lines long, with a massive match statement for decoding all constant pool entry types inside a loop.
**Action:** Extract the body of large loop matches into typed helper functions like `parse_cp_entry` to flatten the structure and keep loop functions brief and understandable.

**[Extracted Class Members Parser]
**Learning:** `parse_class_file` in `crates/duke-classfile/src/parser.rs` handled validating the header (magic, versions), parsing the constant pool, *and* looping through all class members (interfaces, fields, methods, attributes). Doing multiple levels of sequential structural parsing in one function makes it a God Function.
**Action:** Extract logical sections of file format parsing. Parse the header and constant pool first, then delegate to a `parse_class_members` function for the rest to clearly delineate the phases of decoding.

**Extract Enum Variant Matchers**
**Learning:** `crates/duke-bytecode/src/cfg.rs` contained multiple massive `match` blocks repeatedly enumerating the 15+ conditional branch instructions, unconditional jumps, and return instructions to generate CFGs and compute cyclomatic complexity. This duplicated logic and inflated file size.
**Action:** Always extract boolean categorization logic (e.g., `is_conditional_branch()`, `is_return()`) and data extraction logic (`conditional_branch_target()`) into public helper methods directly on the enum (`Instruction`) to DRY up matching code and dramatically flatten calling modules.

**Extract branch matchers**
**Learning:** `build_basic_blocks` in `crates/duke-bytecode/src/basic_block.rs` contained a massive, 60-line inline `match` statement to identify conditional branches, unconditional jumps, and return instructions. This created deep nesting and duplicated logic.
**Action:** Always extract boolean categorization logic and data extraction logic into public helper methods directly on the enum (e.g., `Instruction`) to DRY up matching code and dramatically flatten calling modules. Use `if-else` chains with these helpers instead of massive inline `match` blocks.
**[Fixed clippy::io_other_error]
**Learning:** `std::io::Error::new(std::io::ErrorKind::Other, ...)` is clunky and triggers the `clippy::io_other_error` lint.
**Action:** Replace it with the more concise `std::io::Error::other(...)` to improve readability and conform to idiomatic Rust.

**Extract Subroutine Matchers**
**Learning:** `crates/duke-bytecode/src/cfg.rs` contained duplicated inline `matches!(instr, Instruction::Jsr(_) | Instruction::JsrW(_))` boilerplate which cluttered logic and hid intent.
**Action:** Extract categorization logic into cleanly named helper methods directly on the enum (e.g., `is_subroutine_call()`) to DRY up matching code and flatten the calling modules.

**Extract Struct Building Functions**
**Learning:** `build_class_context` was over 200 lines long, performing all the sequential parsing to populate a large `ClassContext` struct.
**Action:** Extract large sequential parsing chunks (e.g. mapping over methods and fields) into small helper functions returning the parsed components (`build_method_entries`, `build_field_entries`).

**Extract Binary Parsing Headers**
**Learning:** `crates/duke-loader/src/zip.rs` had a `parse_central_directory` function which was very long due to manual reading and variable binding for each field in a ZIP central directory header. This "God Function" approach mixed control-flow iteration with low-level offset-based parsing, increasing cognitive overhead.
**Action:** Extract the inline field extraction logic into a small `CdHeader` struct and a dedicated parsing function `parse_cd_header` to reduce nesting, decouple parsing from collection logic, and flatten the outer loop structure.

**Extract Nested Attribute Loops**
**Learning:** `build_index` in `crates/duke-loader/src/jimage.rs` contained an inline, unbounded `loop` performing offset-based decoding of arbitrary-length location attributes, burying the actual map construction logic.
**Action:** Extract deeply nested binary decoding loops into independent typed structures (like `LocationAttrs`) and helper functions (e.g., `parse_location_attributes`). This eliminates the "Pyramid of Doom" and restores clear separation of concerns between binary chunk decoding and hash map construction.
