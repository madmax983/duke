# 🔭 Vantage: Spec for `java.lang.System.getSecurityManager`

## 👤 User Story
"As a Java Application Developer running existing libraries on Duke, I want `System.getSecurityManager()` to execute without throwing an `Unsupported native` error, so that I can run frameworks like `slf4j` that opportunistically check for a security manager during initialization."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke halts execution when third-party libraries (such as `slf4j-simple`) invoke `java.lang.System.getSecurityManager()`. Many legacy and modern Java libraries perform this check to determine if they are running in a restricted environment. By not supporting this native method, Duke is artificially incompatible with a wide swath of the Java ecosystem, preventing users from adopting Duke for standard enterprise workloads. Implementing this method, even as a simple stub that returns `null` (indicating no security manager is active, which is the standard default behavior in modern Java), immediately unblocks these critical libraries and our own `oss_jar_smoke` test suite.

## 📈 Metric Definition
Success = The `slf4j_simple_smoke_runs_real_jar_bytecode` test suite (the OSS JAR canary) passes end-to-end without crashing on "Unsupported native: java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;".

## 🔍 Gap Analysis
- **Current State:** Duke throws an `Unsupported native` panic when `System.getSecurityManager()` is called, blocking execution.
- **Market/Standard Lib:** The standard JVM provides this native method, and by default (unless specifically configured), it returns `null` indicating no active security manager.
- **The Gap:** We need to implement a native handler for `java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;` in `duke-interpreter` that simply pushes a `null` reference onto the operand stack.

## ✅ Acceptance Criteria
- Must implement the native bridge for `java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;`.
- Must return `null` (representing no active SecurityManager) to the caller.
- Must not introduce any actual SecurityManager logic, permissions checking, or access control contexts.
- The existing `oss_jar_smoke` tests currently blocked by Issue #687 must execute past this native call successfully.

## 🚫 Out of Scope
- Implementing the actual `java.lang.SecurityManager` class or any of its permission-checking methods.
- Support for `System.setSecurityManager()`.
- Access Control Contexts or any actual sandboxing features.
