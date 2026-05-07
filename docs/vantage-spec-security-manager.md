# 🔭 Vantage: Spec for `java.lang.SecurityManager` Support

## 👤 User Story
"As a Backend Developer running standard open-source libraries like SLF4J on Duke, I want the VM to handle `java.lang.System.getSecurityManager()` gracefully, so that my applications can boot without crashing due to missing native methods."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke fails to run many standard third-party libraries (like SLF4J) because they proactively check for a `SecurityManager` at startup. If Duke throws a `NoSuchMethodError` or panics on `System.getSecurityManager()`, we are blocked from running a huge portion of the Java ecosystem. Modern Java applications rarely use SecurityManager (it is deprecated for removal in JEP 411), but libraries still check for it. By stubbing or providing basic support (e.g., returning `null` to indicate no security manager is present, as is standard in modern JVMs by default), we unlock compatibility with thousands of OSS JARs.

## 📈 Metric Definition
Success = A user can run `slf4j-simple` (or any library calling `System.getSecurityManager()`) on Duke without encountering a `NoSuchMethodError` or `UnsatisfiedLinkError`, and the call returns `null`.

## 🔍 Gap Analysis
- **Current State:** Duke is missing the native bridge for `java.lang.System.getSecurityManager()`, preventing standard OSS JARs from starting (tracked in issue #687).
- **Market/Standard Lib:** The standard HotSpot JVM implements this native method and returns a `SecurityManager` instance or `null`. Since Java 17+, it defaults to `null` unless enabled.
- **The Gap:** We need to implement the `getSecurityManager()` method in Duke's standard library/native bridging to unblock third-party library initialization.

## ✅ Acceptance Criteria
- Must implement the native method `java.lang.System.getSecurityManager()Ljava/lang/SecurityManager;`.
- The method must return `null` by default (matching modern JVM default behavior).
- Must unblock the `slf4j-simple` test case in the OSS JAR smoke harness.
- Must not introduce complex permission-checking logic or a full `SecurityManager` implementation (YAGNI).

## 🚫 Out of Scope
- Full implementation of Java's Security Manager permission checks (`FilePermission`, `SocketPermission`, etc.).
- Support for setting a custom SecurityManager via `System.setSecurityManager()`.
