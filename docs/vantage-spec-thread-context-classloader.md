# 🔭 Vantage: Spec for `java.lang.Thread` Context ClassLoader

## 👤 User Story
"As a Java Framework Developer running my code on Duke, I want the VM to support `Thread.getContextClassLoader()` and `setContextClassLoader()`, so that my frameworks (like slf4j or Spring) can dynamically locate resources and isolate dependencies on a per-thread basis."

## ❓ The "So What?"
What business problem does this solve?
It is the current explicit blocker for running the `slf4j-simple` real-world JAR compatibility test. Modern Java architectures heavily rely on Thread Context ClassLoaders (TCCL) for dependency injection and resource loading, especially when the framework classes are loaded by a parent classloader but need to load application classes. Without TCCL support, Duke is limited to basic standalone applications. Supporting it unlocks compatibility with the broader Java ecosystem.

## 📈 Metric Definition
Success = The `slf4j-simple` smoke test progresses past `System.getSecurityManager()` without crashing on `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;` returning null inappropriately or failing to execute.

## 🔍 Gap Analysis
- **Current State:** Duke has partial internal implementation for threads. The internal memory slot `THREAD_CONTEXT_CLASS_LOADER_SLOT` exists, and the `native_thread_get_context_class_loader` and `native_thread_set_context_class_loader` functions are stubbed in `native.rs` and registered in `stdlib.rs`. However, it appears there might be missing logic for inheriting the context classloader during thread creation or returning appropriate default values, which is causing compatibility issues.
- **Market/Standard Lib:** The standard JVM `java.lang.Thread` automatically inherits the context classloader of the thread that created it. If not set, it defaults to the system classloader.
- **The Gap:** We need to ensure that the native bridge logic correctly inherits the context classloader from the parent thread at thread creation (`native_thread_init` and related functions), and correctly exposes this to Java code.

## ✅ Acceptance Criteria
- Must fully implement native bridges for `getContextClassLoader` and `setContextClassLoader`.
- Must inherit the context classloader from the parent thread at the time of thread creation.
- Must fall back to the system classloader if no context classloader has been explicitly set or inherited.

## 🚫 Out of Scope
- Implementing full custom `ClassLoader` mechanics or `URLClassLoader` behavior. This spec is strictly about exposing and managing the context classloader reference on the `Thread` object.
- Network-based class loading.
