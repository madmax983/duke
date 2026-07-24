**Refactor Ldc instructions**
**Learning:** Different variants of identical instructions (`Ldc`, `LdcW`, `Ldc2W`) often have duplicate logic which leads to deeply nested repetition.
**Action:** Combine match arms and extract shared lookup mechanisms to adhere to DRY principles.
