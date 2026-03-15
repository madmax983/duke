**[Guard Clauses]
**Learning:** Found an `assert!(events.len() >= 1)` which is less expressive than `assert!(!events.is_empty())` and warned by clippy.
**Action:** Use `!is_empty()` instead of checking `len() >= 1` for empty checks.

**[Readability Smell]
**Learning:** Found unused variables prefix with `_` triggering unused warnings because I removed the underscore but forgot to put `#[allow(unused_variables)]`.
**Action:** Make sure to use `#[allow(unused_variables)]` if needed, or simply leave the `_` prefix if not used to avoid warnings.

**[Clippy Refactoring on Primitives]**
**Learning:** Found clippy suggesting `.cast_signed()` on primitives when resolving wrap/truncation lint errors, but `.cast_signed()` only exists for raw pointers, breaking compilation.
**Action:** When clippy says "casting `uX` to `iX` may wrap around the value... use `.cast_signed()` instead", ignore the hallucinated method. Instead, use `as` casting and `#[allow(clippy::cast_possible_wrap)]`.

**[Truncation Error Handling in GC]**
**Learning:** Found a clippy suggestion to resolve truncation errors by replacing `as usize` with `usize::try_from(r).unwrap_or(usize::MAX)` in duke-gc, breaking the fail-fast behavior.
**Action:** Truncation errors in a GC memory reference should either gracefully return a `VmError` or fail fast with `.unwrap()`. Masking the error by falling back to `usize::MAX` is a logic bug that crashes the GC in obscure ways.
