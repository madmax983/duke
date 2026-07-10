# 🔭 Vantage: Spec for Full Reflection Access Flags

👤 **User Story:** "As a Java Developer relying on libraries like Gson or Spring, I want reflection methods like `Class.getModifiers()`, `Class.isInterface()`, and `Field.getModifiers()` to accurately report all access flags (e.g. transient, final, interface, abstract), so that serialization logic and dependency injection behaviors (like skipping transient fields or mocking interfaces) function correctly and do not result in silent data loss or runtime errors."

**So What? / Business Problem:** Current implementation only reports `ACC_PUBLIC`, masking critical metadata. This breaks serialization libraries (e.g. Gson won't skip transient fields) and frameworks relying on interface/abstract checks. Accurate metadata is a table-stakes capability for running any real-world Java workload.

**Success Metric:** `Class.getModifiers()`, `Class.isInterface()`, and `Field.getModifiers()` return bitmasks exactly matching the original bytecode's flags, allowing the `gson` canary and Spring Boot apps to properly identify interfaces and skip transient/final fields.

✅ **Acceptance Criteria:**
- `Class.getModifiers()` and `Class.isInterface()` must report the real access flags (including interface, abstract, final, etc.), not just `ACC_PUBLIC`.
- `Field.getModifiers()` must report the real access flags (including transient and final), not just `ACC_PUBLIC`.

🚫 **Out of Scope:**
- Real-time execution (Phase 2).
