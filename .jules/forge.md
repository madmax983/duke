**[Guard Clauses]
**Learning:** Found an `assert!(events.len() >= 1)` which is less expressive than `assert!(!events.is_empty())` and warned by clippy.
**Action:** Use `!is_empty()` instead of checking `len() >= 1` for empty checks.

**[Readability Smell]
**Learning:** Found unused variables prefix with `_` triggering unused warnings because I removed the underscore but forgot to put `#[allow(unused_variables)]`.
**Action:** Make sure to use `#[allow(unused_variables)]` if needed, or simply leave the `_` prefix if not used to avoid warnings.
## 2026-03-16 - Extracted duplicated invoke_cb
**Learning:** Found an inline `invoke_cb` closure duplicated 5 times with identical bodies to construct a callback. Creating an inline macro using `macro_rules!` cleans this up nicely without needing a free function to explicitly list captured vars.
**Action:** Look out for code duplication and use `macro_rules!` for DRYing up closure logic where typical function extraction struggles due to context capture.

**[Iterator Chains]
**Learning:** Manual `for` loops with pre-allocated `Vec::with_capacity` pushing parsed elements are verbose and common.
**Action:** Replace them with idiomatic iterator chains `(0..count).map(|_| ...).collect::<Result<Vec<_>, _>>()?` to safely propagate errors and improve readability, relying on `collect` to infer capacity from the `ExactSizeIterator`.

**[Extracting Repeated Code]
**Learning:** Found identical `while i + 1 < fields.len()` loops used to iterate over key-value pairs in HashMap natives, and similar logic in HashSet.
**Action:** Extracted these loops into simple helper functions `find_hashmap_entry_index` and `find_hashset_entry_index` which return `Option<usize>`, simplifying five separate native functions and removing `mut i` boilerplate.

**[Extracting Native Print Functions]
**Learning:** Attempted to use `macro_rules!` to extract identical boilerplate for `native_print_*` and `native_println_*` functions. This violated the "Ask first" constraint for complex macros and failed compilation due to a hallucinated `cast_unsigned()` method when trying to clean up an `as u32` cast for clippy.
**Action:** When constrained against macros, use small generic helper functions (e.g., passing a closure or function pointer for the formatting) instead of macros to enforce DRY without hiding logic. Also, remember `cast_unsigned` is not a standard Rust method for `i32`; stick to `as u32`.
