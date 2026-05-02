# 🔭 Vantage: Spec for System.getSecurityManager() Support

## 👤 User Story
"As a Java Application Developer running standard open-source libraries on Duke, I want the JVM to gracefully handle `java.lang.System.getSecurityManager()` calls so that frameworks relying on standard security manager checks can execute without crashing the VM due to missing native methods."

## ❓ The "So What?"
What business problem does this solve?
Currently, running real-world, third-party OSS JARs (like `slf4j-simple`) on Duke is blocked. Many standard Java libraries perform defensive checks by invoking `System.getSecurityManager()` to determine if they need to enforce sandbox restrictions. Because Duke entirely lacks the native bridge for this method, these libraries immediately crash with an `Unsupported native` error. Fixing this unblocks the execution of foundational Java ecosystem components (such as logging frameworks and application servers) on Duke, directly expanding its real-world utility from a toy project to a capable execution environment.

## 📈 Metric Definition
Success = The `tests/fixtures/oss-jars/slf4j-simple` test execution unblocks, specifically moving past the `java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;` missing native error.

## 🔍 Gap Analysis
- **Current State:** Duke JVM interpreter crashes immediately upon encountering `java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;` with an "Unsupported native" error. The `oss_jar_smoke` test for `slf4j-simple` is explicitly ignored/blocked by Issue #687 due to this.
- **Market/Standard Lib:** Modern JVMs support `System.getSecurityManager()` by returning either an active `SecurityManager` instance or `null` if no security manager is installed (which is the default behavior in almost all modern Java applications).
- **The Gap:** Duke needs a native method implementation for `System.getSecurityManager()` that mimics the default JVM behavior of running without a security manager, returning `null`.

## ✅ Acceptance Criteria
- Must implement the native bridge for `java.lang.System.getSecurityManager()`.
- The native bridge must return `null` (or the equivalent JVM representation of a null object reference).
- Must prevent the interpreter from crashing with an "Unsupported native" error when the method is invoked.
- Must allow the `oss_jar_smoke` future-canary test for `slf4j-simple` to proceed past the `getSecurityManager` call.

## 🚫 Out of Scope
- Actually implementing a functional `java.lang.SecurityManager` and the complex access control architecture (`AccessController`, `ProtectionDomain`, etc.).
- Parsing or enforcing `java.policy` files.
- Modifying standard library security checks elsewhere in the JDK.