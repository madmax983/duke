# 🔭 Vantage: Spec for `System.getSecurityManager()` Stubbing

## 👤 User Story
"As a developer porting standard Java libraries (like slf4j) to Duke, I want the JVM to safely handle calls to `java.lang.System.getSecurityManager()`, so that legacy library initialization code doesn't crash the VM with unsupported native method errors."

## ❓ The "So What?"
What business problem does this solve?
Many mature, widely-used Java libraries (such as `slf4j`, which is a fundamental logging facade) were built in an era where the Java Security Manager was a core part of the platform. Even though modern Java environments often run without a Security Manager (and it is deprecated in newer Java versions), legacy libraries still conditionally check for its presence at startup via `System.getSecurityManager()`. Currently, Duke panics on this unimplemented native method, blocking the use of these critical ecosystem libraries (tracked as issue #687). Providing a safe stub unblocks real-world framework compatibility (like our OSS JAR smoke tests).

## 📈 Metric Definition
Success = The OSS JAR smoke test for `slf4j-simple-2.0.13` successfully executes the `java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;` native call and continues executing subsequent bytecode without a native resolution panic.

## 🔍 Gap Analysis
- **Current State:** Duke throws an "Unsupported native" error when encountering `java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;`.
- **Market/Standard Lib:** Standard JVMs return a `SecurityManager` object if configured, or `null` if no security manager is installed. For most modern, standard server applications, it returns `null`.
- **The Gap:** We need a minimal native bridge implementation in `duke-interpreter` for `System.getSecurityManager()` that simply returns `null` (or the internal equivalent like a null object reference), accurately mimicking a JVM running without a configured Security Manager.

## ✅ Acceptance Criteria
- Must implement the native method `java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;` in the `duke-interpreter` crate.
- Must return a `null` reference, simulating an environment with no security manager.
- The `slf4j-simple` OSS JAR smoke test must bypass this specific unsupported native error.

## 🚫 Out of Scope
- Actually implementing a functional Java Security Manager or permission checking system.
- Implementing `System.setSecurityManager()`.
- Supporting other deprecated `java.security` APIs beyond what is strictly necessary to return `null` here.
