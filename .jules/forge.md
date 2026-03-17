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
