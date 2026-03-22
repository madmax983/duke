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

**[Preserving Jump Tables in Refactors]
**Learning:** Breaking a massive `match` statement in a hot path (like a bytecode decoder) into multiple sequential helper functions ruins the compiler's ability to generate an O(1) jump table, causing a severe performance regression.
**Action:** When fixing `clippy::too_many_lines` on a massive `match`, keep the massive `match` statement intact and use `#[allow(clippy::too_many_lines)]`. Only extract the complex logic *inside* specific arms (like `TABLESWITCH` or `LOOKUPSWITCH`) into helper functions.

**[Extracting Native Print Boilerplate]
**Learning:** Generating full function signatures inside `macro_rules!` (e.g. `define_native_print!`) breaks IDE navigability ("Go to Definition") and readability, even if it saves lines.
**Action:** When deduplicating boilerplate across multiple functions in Rust (e.g., argument extraction in `duke-interpreter` native functions), prefer using inline `macro_rules!` macros (e.g., `extract_print_arg!`) to handle the repetitive inner logic rather than generating entire function signatures. This preserves IDE features and keeps function signatures explicit.
