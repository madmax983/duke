# 🔭 Vantage: Spec for Thread Context ClassLoader

## 👤 User Story
"As an Enterprise Java Developer running my code on Duke, I want the VM to support Thread Context ClassLoaders (TCCL), so that modern logging frameworks like SLF4J and dependency injection frameworks like Spring Boot can dynamically load resources without panicking."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke lacks support for `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;`, which is the exact blocker preventing standard OSS JARs like `slf4j-simple` from booting. Many enterprise Java libraries rely on the Thread Context ClassLoader to load configuration files or SPI implementations dynamically, bypassing the system classloader. Without this capability, Duke is incompatible with a massive segment of real-world Java libraries. Supporting TCCL is a foundational requirement for ecosystem compatibility and executing modern Java workloads on Duke.

## 📈 Metric Definition
Success = The `slf4j-simple` OSS JAR smoke test progresses past the `java.lang.Thread.getContextClassLoader()` check without crashing, proving the native bridge correctly stores and retrieves the context classloader.

## 🔍 Gap Analysis
- **Current State:** Duke fails on `slf4j-simple` because it encounters a native method invocation for `Thread.getContextClassLoader()` that is not implemented.
- **Market/Standard Lib:** Standard JVMs assign a context classloader to every thread upon creation (usually inheriting from the parent thread or the system classloader). The `Thread` class provides native getters and setters for this property.
- **The Gap:** We need to provide native bridges for `java.lang.Thread.getContextClassLoader()` (and subsequently `setContextClassLoader()`) and back these with thread-local storage in Duke's threading subsystem.

## ✅ Acceptance Criteria
- Must implement native bridge for `java.lang.Thread.getContextClassLoader()`.
- Must implement native bridge for `java.lang.Thread.setContextClassLoader()`.
- The `slf4j-simple` OSS JAR smoke test must progress past the TCCL check.
- Thread Context Classloader state must be properly isolated per-thread in the interpreter.

## 🚫 Out of Scope
- Full custom classloader implementation (only the Thread context storage/retrieval mechanism is needed).
- Fixing subsequent blockers in `slf4j-simple` after TCCL (this spec focuses *only* on the TCCL capability).
