**Refactoring nested Options**
**Learning:** Flattening nested `Option` closures mapping to a single output using `let Some(Some(..)) = ... else { return None; };` drastically improves readability without altering logic.
**Action:** Use let-else guard clauses for Option chains when appropriate.
