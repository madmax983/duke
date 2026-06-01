# 🔭 Vantage: Spec for Thread Context ClassLoader

## 👤 User Story
"As a Java Application Developer running my code on Duke, I want the VM to support `java.lang.Thread.getContextClassLoader()`, so that my frameworks (like slf4j or Spring) can dynamically load classes from the appropriate classloader, even if the framework classes themselves were loaded by a parent classloader."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke's threading model lacks support for Thread Context ClassLoaders (TCCL). Many popular Java frameworks (such as slf4j, Spring Boot, JAXB, JNDI) rely heavily on TCCL to load user-provided application classes or resources from a framework library that was loaded by the system classloader. Without TCCL, these frameworks encounter `ClassNotFoundException` when attempting to discover application-specific implementations (e.g., SLF4J failing to find the underlying logger implementation). This is currently blocking real-world JAR compatibility, specifically `slf4j-simple`. Supporting TCCL is critical to achieving compatibility with standard, complex enterprise Java software.

## 📈 Metric Definition
Success = A Java framework (such as `slf4j-simple`) running on Duke can successfully retrieve the Thread Context ClassLoader via `Thread.currentThread().getContextClassLoader()` and use it to load application classes that are not visible to the framework's own classloader.

## 🔍 Gap Analysis
- **Current State:** Duke's threading subsystem (`crates/duke-interpreter/src/threading.rs`) and native method bridges do not implement or expose the `contextClassLoader` field on `java.lang.Thread`.
- **Market/Standard Lib:** The standard JVM provides `getContextClassLoader()` and `setContextClassLoader(ClassLoader cl)` on the `java.lang.Thread` class, initializing it to the system classloader (or parent thread's TCCL) by default.
- **The Gap:** We need to update the internal thread representation in `duke-interpreter` to store a reference to the `ClassLoader` and implement the native bridge methods for `getContextClassLoader` and `setContextClassLoader` in `java.lang.Thread`.

## ✅ Acceptance Criteria
- Must implement the native bridges required by `java.lang.Thread.getContextClassLoader()` and `java.lang.Thread.setContextClassLoader(ClassLoader cl)`.
- A newly created thread must inherit the context classloader from its parent thread.
- The main thread must have its context classloader initialized to the default system classloader.
- Must throw a `SecurityException` if a security manager exists and its `checkPermission` method doesn't allow getting or setting the context classloader (though full security manager implementation might be a separate concern, the basic check structure should be considered if applicable).

## 🚫 Out of Scope
- Implementing full `SecurityManager` permission checks (if not strictly necessary to get `slf4j` unblocked, simple stubs might suffice initially).
- Implementing other custom classloaders beyond what is needed to expose the TCCL property on Threads.
