# Spec: Thread Context ClassLoader

👤 **User Story:** "As a Java Developer, I want to get and set the context ClassLoader for a thread, so that dynamically loaded frameworks like SLF4J can correctly resolve user-supplied implementations."

**"So What?" (Business Value):**
Enterprise software relies heavily on context class loaders to load resources, configuration files, and dynamically registered service providers (e.g., SPI, logging implementations). Without this functionality, the JVM cannot run standard enterprise Java libraries, blocking its adoption for real-world production systems.

**Metric Definition:**
- Success = The `slf4j-simple` test fixture progresses past `java/lang/Thread.getContextClassLoader()` without a native method resolution failure.
- Success = `Thread.getContextClassLoader` returns the correctly scoped ClassLoader for the current thread.

**Gap Analysis:**
The current runtime successfully navigates basic security checks (`System.getSecurityManager()` and `AccessController.doPrivileged`) but lacks the internal bookkeeping and native method implementations for `Thread.getContextClassLoader()`. This causes a hard blocker for Phase 117 integration tests.

✅ **Acceptance Criteria:**
- `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;` must be implemented and accessible during bytecode execution.
- `java/lang/Thread.setContextClassLoader(Ljava/lang/ClassLoader;)V` must be implemented to allow frameworks to mutate the context.
- The reference to the Context ClassLoader must be properly scoped to the individual Thread instance.
- The `oss_jar_smoke` future-canary test for SLF4J must advance past this specific missing method.

🚫 **Out of Scope:**
- Full implementation of `SecurityManager.checkPermission` for `getContextClassLoader` (assume open access for this milestone).
- Parent thread inheritance of the context class loader during `Thread` construction (Phase 2).
