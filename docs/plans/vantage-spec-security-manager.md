# 🔭 Vantage: Spec for SecurityManager Stub (`java.lang.System.getSecurityManager`)

## 👤 User Story
"As a user trying to run a third-party Java library (like `slf4j-simple`) on Duke, I want the JVM to handle calls to `java.lang.System.getSecurityManager()` gracefully, so that the library can initialize properly without crashing due to unsupported natives."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke crashes when third-party libraries attempt to verify permissions using `System.getSecurityManager()`. Since Duke does not currently enforce a Java Security Model (and even modern Java has deprecated it for removal), failing here completely blocks real-world libraries from functioning. Implementing a minimal stub that returns `null` (indicating no security manager is present) unlocks compatibility with massive parts of the Java ecosystem, turning a liability (crashing on boot) into a capability (running real libraries).

## 📈 Metric Definition
Success = The `slf4j_simple_smoke_runs_real_jar_bytecode` test (Issue #687) transitions from `#[ignore]` to passing, proving that Duke can execute the library without hitting the `getSecurityManager` natively-unimplemented error.

## 🔍 Gap Analysis
- **Current State:** Duke has no implementation for `System.getSecurityManager()`.
- **Market/Standard Lib:** The standard JVM provides this method. When no security manager is set, it returns `null`. Many libraries proactively check this during static initialization to decide whether they need to wrap operations in `AccessController.doPrivileged()`.
- **The Gap:** We need a native bridge implementation for `java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;` that simply returns a `null` reference slot.

## ✅ Acceptance Criteria
- Must implement native method `java.lang.System.getSecurityManager()Ljava/lang/SecurityManager;`.
- The native method must return `null` to indicate no security manager is installed.
- The `slf4j_simple_smoke_surfaces_first_missing_native_explicitly` test behavior will change since this native will now exist (may need to be updated to target the *next* missing capability or removed).
- The `slf4j_simple_smoke_runs_real_jar_bytecode` should progress further (or pass).

## 🚫 Out of Scope
- Full implementation of `java.lang.SecurityManager` and actual permission checks.
- Setting a custom Security Manager (`System.setSecurityManager()`).
- `AccessController.doPrivileged` implementation.
