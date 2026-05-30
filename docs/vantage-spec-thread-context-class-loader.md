# 🔭 Vantage: Spec for Thread.getContextClassLoader()

## 👤 User Story
"As an Application Developer, I want Duke to support \`java/lang/Thread.getContextClassLoader()\`, so that standard libraries like SLF4J can load service providers and initialize properly."

## ❓ The "So What?"
What business problem does this solve?
Currently, our JVM (Duke) is blocked on running real-world third-party libraries (like `slf4j-simple`) because it halts when executing `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;`. Frameworks depend heavily on the context class loader to discover plugins, load service providers, and initialize logging implementations dynamically. Unblocking this will allow us to run industry-standard Java libraries, increasing Duke's compatibility and potential market adoption as a viable JVM replacement.

## 📈 Metric Definition
Success = The `slf4j-simple` OSS smoke test progresses past `Thread.getContextClassLoader()` without panicking, moving the CI pipeline forward.

## 🔍 Gap Analysis
- **Current State:** Duke lacks an implementation for `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;` blocking the `slf4j-simple` OSS smoke test (Phase 117 + #695).
- **Market/Standard Lib:** Standard JVMs (HotSpot, OpenJ9) provide a context class loader per thread, which defaults to the system class loader and is essential for service discovery (e.g., `ServiceLoader`).
- **The Gap:** We need a functioning implementation for `Thread.getContextClassLoader()` to unblock standard framework initialization.

## ✅ Acceptance Criteria
- Must implement `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;`.
- Must allow the `slf4j-simple` OSS JAR smoke test to progress beyond this method.
- Must not cause memory leaks or panics.

## 🚫 Out of Scope
- Full thread-local inheritance of context class loaders.
- `setContextClassLoader` implementation (unless strictly necessary for the smoke test).
