# 🔭 Vantage: Spec for SecurityManager Native Stubs

## 👤 User Story
"As a Java Application Developer running my code on Duke, I want the VM to safely handle `System.getSecurityManager()` checks, so that popular libraries like SLF4J can initialize without throwing unsupported native exceptions."

## ❓ The "So What?"
What business problem does this solve?
Currently, our primary OSS smoke test JAR `slf4j-simple` is failing initialization because it attempts to query the `SecurityManager` during its setup sequence. While modern Java is deprecating the `SecurityManager`, thousands of legacy libraries still defensively query it (e.g., `System.getSecurityManager()`). Without stubbing this native interaction, the VM will crash on these libraries, limiting Duke's compatibility with the broader Java ecosystem. Supporting a "null" security manager safely unblocks the execution path for these common patterns.

## 📈 Metric Definition
Success = The OSS smoke test for `slf4j-simple` advances past the `Unsupported native: java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;` error phase.

## 🔍 Gap Analysis
- **Current State:** Duke throws an `Unsupported native` error when `System.getSecurityManager()` is called.
- **Market/Standard Lib:** Most JVMs, especially those not running strict policies, return `null` for the security manager, or implement it fully.
- **The Gap:** We need a minimal native bridge for `java.lang.System.getSecurityManager()` that safely returns `Slot::Null`, correctly satisfying the caller that no restrictive security manager is present, without needing to implement the full complex access control logic.

## ✅ Acceptance Criteria
- Must implement the native stub for `java.lang.System.getSecurityManager()Ljava/lang/SecurityManager;`.
- Must consistently return `Slot::Null` to indicate no security manager is active.
- The OSS JAR smoke test for `slf4j-simple` must progress past the current missing native exception.

## 🚫 Out of Scope
- Full implementation of `java.lang.SecurityManager` and related exception throwing.
- Support for `java.security.AccessController` or permission-based checks (like `doPrivileged`).
- Parsing or enforcing Java security policies (`.policy` files).
