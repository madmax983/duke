# 🔭 Vantage: Spec for `java.lang.System.getSecurityManager()`

## 👤 User Story
"As a Java Application Developer using open-source libraries (like SLF4J) on Duke, I want `System.getSecurityManager()` to return `null` by default, so that my applications can bypass legacy security manager checks and execute successfully."

## ❓ The "So What?"
**What business problem does this solve?**
Many prevalent third-party libraries (like SLF4J, Spring, and others) perform defensive checks during initialization to see if a `SecurityManager` is present. If it is, they use privileged execution blocks. Currently, Duke throws an "Unsupported native" exception when these checks occur, blocking the execution of fundamental, real-world libraries.
Since Duke targets Java SE 21 semantics—where the `SecurityManager` is deprecated for removal (JEP 411) and disabled by default—we don't need to build a complex, legacy permissions model. By simply stubbing this method to return `null`, we unlock a massive amount of ecosystem compatibility with virtually zero complexity cost.

## 📈 Metric Definition
Success = The `oss_jar_smoke` test for `slf4j-simple` completes execution successfully, meaning `System.getSecurityManager()` resolves and returns `null` without crashing the JVM.

## 🔍 Gap Analysis
- **Current State:** Duke fails fast with an unsupported native method panic when `System.getSecurityManager()` is invoked (Issue #687).
- **Market/Standard Lib:** OpenJDK 21 returns `null` for `getSecurityManager()` by default unless explicitly opted-in via command-line flags.
- **The Gap:** We need a minimal native bridge for `java.lang.System.getSecurityManager()Ljava/lang/SecurityManager;` that pushes a null reference onto the operand stack.

## ✅ Acceptance Criteria
- Must implement the native method bridge for `java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;`.
- Must return a null reference to the caller.
- Must un-ignore and successfully pass the `slf4j_simple_smoke_runs_real_jar_bytecode` test in `oss_jar_smoke.rs`.

## 🚫 Out of Scope
- Implementing `java.lang.SecurityManager` behavior, policies, or permission checks.
- Supporting the `-Djava.security.manager` flag.
