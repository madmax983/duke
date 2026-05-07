1. Add new string extraction helper functions that match the *exact* semantics of the existing code, to avoid runtime errors or behavioral changes. The original matches often return `String::new()` or `None` on failure, instead of `Error::TypeMismatch`.
2. Use python to safely substitute only matching boilerplate blocks.
3. Validate by tests.
4. Clean up any python scripts left in the workspace.
5. Code review and submit.
