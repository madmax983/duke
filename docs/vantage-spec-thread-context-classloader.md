# 🔭 Vantage: Spec for Thread Context ClassLoader

## 👤 User Story
"As a Java framework maintainer (e.g., SLF4J, Spring), I want threads to provide a Context ClassLoader (TCCL), so that my framework can dynamically load user-provided application classes without requiring them to be on the system or framework classpath."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke is blocked from running the `slf4j-simple` OSS JAR smoke test because the framework attempts to dynamically load binding classes via `Thread.currentThread().getContextClassLoader()`. Modern Java applications, application servers, and SPIs (Service Provider Interfaces) rely heavily on TCCL to break strict classloader hierarchy constraints. Without it, standard open-source libraries will crash on initialization, blocking Duke from executing real-world Java ecosystem code.

## 📈 Metric Definition
Success = The `slf4j-simple` smoke test progresses past `java.lang.Thread.getContextClassLoader()` without a native method not found error, returning a valid ClassLoader reference (or null for bootstrap) that can be used to resolve classes.

## 🔍 Gap Analysis
- **Current State:** Duke lacks the native implementation for `java.lang.Thread.getContextClassLoader()` and the internal state to track the `contextClassLoader` on the `Thread` object.
- **Market/Standard Lib:** The standard JVM provides `getContextClassLoader()` and `setContextClassLoader(ClassLoader)` on `java.lang.Thread`. By default, new threads inherit the TCCL of their parent thread. The primordial thread's TCCL is typically set to the system classloader.
- **The Gap:** We need to add internal state to Duke's `Thread` representation to hold a reference to a `ClassLoader` instance, initialize it properly (inheriting from the parent thread, or setting it to the system classloader for the main thread), and implement the native bridge for `getContextClassLoader()`.

## ✅ Acceptance Criteria
- Must implement the native method `java.lang.Thread.getContextClassLoader()Ljava/lang/ClassLoader;`.
- Must implement the native method `java.lang.Thread.setContextClassLoader(Ljava/lang/ClassLoader;)V` (or handle the field assignment in Java space if Duke manages it there).
- The main thread must have its context class loader initialized to the system class loader upon startup.
- Child threads must inherit the context class loader of their parent thread at the time of creation.
- Must not panic when `getContextClassLoader()` is called.

## 🚫 Out of Scope
- Implementing full `SecurityManager` permission checks for `getContextClassLoader` or `setContextClassLoader` (since SecurityManager is currently bypassed/stubbed in Duke).
