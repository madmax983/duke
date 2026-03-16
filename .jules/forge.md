**[Guard Clauses]
**Learning:** Found an `assert!(events.len() >= 1)` which is less expressive than `assert!(!events.is_empty())` and warned by clippy.
**Action:** Use `!is_empty()` instead of checking `len() >= 1` for empty checks.

**[Readability Smell]
**Learning:** Found unused variables prefix with `_` triggering unused warnings because I removed the underscore but forgot to put `#[allow(unused_variables)]`.
**Action:** Make sure to use `#[allow(unused_variables)]` if needed, or simply leave the `_` prefix if not used to avoid warnings.
