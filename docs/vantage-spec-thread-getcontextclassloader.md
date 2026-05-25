# 🔭 Vantage: Spec for Thread.getContextClassLoader() Compatibility

## 👤 User Story
"As a Java developer running a real `.jar` via `duke exec`, I want `java.lang.Thread.getContextClassLoader()` to return a valid classloader, so that common libraries such as `slf4j-simple` can initialize through their standard JDK compatibility path instead of failing before application code runs."

## ❓ The "So What?"
What business problem does this solve?
The `slf4j-simple` real-world JAR smoke harness now progresses past `java/lang/System.getSecurityManager()` and `java.security.AccessController.doPrivileged(...)` and stops on `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;`. That leaves the end-to-end `slf4j_simple_smoke_runs_real_jar_bytecode` canary ignored. Supporting the `getContextClassLoader` compatibility path allows Duke to execute common older libraries that rely on it to find application resources, even when a complex classloader hierarchy isn't needed.

## 📈 Metric Definition
Success = `java.lang.Thread.getContextClassLoader()` is implemented and returns a valid ClassLoader instance, and the OSS smoke harness blocker string in `README.md` progresses past `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;`.

## 🔍 Gap Analysis
- **Current State:** Duke fails on `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;` when executing `slf4j-simple`.
- **Market/Standard Lib:** HotSpot, OpenJ9, and GraalVM all implement `Thread.getContextClassLoader()`. By default, it returns the system class loader for the main thread.
- **The Gap:** We need to provide a native implementation for `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;` that returns a reasonable default (like the system class loader) to unblock library initialization.

## ✅ Acceptance Criteria
- `java.lang.Thread.getContextClassLoader()` is implemented as a native method in Duke.
- For the main thread, it returns an instance of a ClassLoader (e.g., the system classloader).
- The vendored `slf4j-simple-2.0.13.jar` method that calls `Thread.getContextClassLoader()` executes without failing.
- `crates/duke-interpreter/tests/oss_jar_smoke.rs::slf4j_simple_smoke_surfaces_next_missing_capability_explicitly` progresses past `Thread.getContextClassLoader()`.
- Integration tests `cargo test -p duke-interpreter --test oss_jar_smoke` pass on the implementation branch.

## 🚫 Out of Scope
- Full implementation of a complex, hierarchical Java ClassLoader architecture.
- `Thread.setContextClassLoader()` if it's not strictly required by the current SLF4J blocker.
- InheritableThreadLocal classloaders.
