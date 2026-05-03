# 🔭 Vantage: Spec for Security Manager Stub

## 👤 User Story
"As a Java Developer running legacy or third-party libraries (like slf4j) on Duke, I want `System.getSecurityManager()` to execute without throwing an unsupported native error, so that my application can initialize and I don't have to rewrite upstream libraries."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke lacks an implementation for `System.getSecurityManager()`. Many ubiquitous third-party libraries (like `slf4j-simple`) check for the presence of a Security Manager during static initialization. Because Duke throws an unsupported native error, these libraries fail to load entirely, preventing Duke from executing real-world JARs. By providing a stub implementation that simply returns `null`, we unblock a massive ecosystem of existing Java code without taking on the monumental (and deprecated) task of implementing a full Java security model. This enables the OSS smoke tests to pass and proves Duke's viability for standard workloads.

## 📈 Metric Definition
Success = The `oss_jar_smoke` test suite can run end-to-end without crashing on the `System.getSecurityManager()` native call, and outputs the expected `INFO` log line.

## 🔍 Gap Analysis
- **Current State:** The Duke interpreter fails with a native method not found error when it encounters the `System.getSecurityManager()` invocation.
- **Market/Standard Lib:** In modern standard JVMs (Java 17+), the Security Manager is deprecated for removal (JEP 411) and `System.getSecurityManager()` typically returns `null` unless explicitly enabled via command-line flags.
- **The Gap:** Duke needs a native stub for `System.getSecurityManager()` that unconditionally returns `null`.

## ✅ Acceptance Criteria
- Must implement the native bridge for `System.getSecurityManager()` that returns `null` (an empty reference).
- Must unblock libraries that check `System.getSecurityManager() == null`.

## 🚫 Out of Scope
- Actually implementing a functional Security Manager or enforcing access control policies.
- Implementing related Security Manager methods like `setSecurityManager()` or `checkPermission()`.
