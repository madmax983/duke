# 🔭 Vantage: Spec for `Thread.getContextClassLoader()` Implementation

## 👤 User Story
"As a Java Application Developer using complex logging or dependency injection frameworks on Duke, I want the JVM to support `Thread.getContextClassLoader()`, so that my libraries can dynamically discover and load classes and resources relative to the executing thread, regardless of the system classpath."

## ❓ The "So What?"
What business problem does this solve?
Currently, the Duke JVM explicitly blocks when encountering `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;`. This specific native method is heavily relied upon by widely used ecosystem libraries like SLF4J, Spring, Tomcat, and essentially any modular framework that separates API from implementation. These libraries use the thread context classloader (TCCL) to locate implementation classes that might not be visible from the library's own classloader. Without supporting TCCL, Duke cannot successfully initialize modern, real-world dependencies (as evidenced by the failure in the `slf4j-simple` OSS JAR smoke test), rendering the JVM incapable of running the vast majority of enterprise Java software. Supporting it unlocks the ability to load and run standard open-source library stacks.

## 📈 Metric Definition
Success = The `oss_jar_smoke` test progresses past the `Thread.getContextClassLoader()` blocker. Specifically, the test `slf4j_simple_smoke_runs_real_jar_bytecode` in `crates/duke-interpreter/tests/oss_jar_smoke.rs` must no longer fail with the `Unsupported native: java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;` error.

## 🔍 Gap Analysis
- **Current State:** Duke throws an `Unsupported native` error when `Thread.getContextClassLoader()` is invoked. Issue #695 implemented `System.getSecurityManager()` and `AccessController.doPrivileged(...)`, making TCCL the very next bottleneck.
- **Market/Standard Lib:** Standard HotSpot/OpenJDK implementations maintain a `contextClassLoader` field on `java.lang.Thread` objects. This field defaults to the system classloader for the main thread and is inherited by child threads upon creation.
- **The Gap:** We need to provide the native bridge implementation for `Thread.getContextClassLoader()`. We also need to ensure that when a Java `Thread` object is initialized, its context classloader field is populated correctly (either inherited from the parent thread or set to the system class loader for the main thread) and we likely need the corresponding setter, `setContextClassLoader`.

## ✅ Acceptance Criteria
- Must implement the native method `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;`.
- The `Thread` class initialization logic must ensure the context classloader field is properly populated. The main thread's TCCL must be initialized to the default system classloader.
- New threads must inherit the context classloader of the thread that created them.
- Must implement `java/lang/Thread.setContextClassLoader(Ljava/lang/ClassLoader;)V` to allow frameworks to temporarily swap the classloader.
- The `slf4j-simple` OSS smoke test must unblock and progress to the next capability gap (or pass entirely).

## 🚫 Out of Scope
- Full implementation of `URLClassLoader` (tracked separately).
- Implementing complete ClassLoader security policies (e.g., checking `RuntimePermission("getClassLoader")`). Phase 1 will just return the loader without checking permissions.
