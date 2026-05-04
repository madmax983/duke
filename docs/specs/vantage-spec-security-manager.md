# Product Spec: SecurityManager Support

👤 **User Story:**
As a Java Developer running third-party libraries (like `slf4j`) on Duke JVM, I want `System.getSecurityManager()` to execute cleanly, so that my library's static initialization can complete and my application can start.

❓ **So What? (Business Problem):**
The vast majority of real-world Java applications and standard libraries include early checks for an active `SecurityManager` (e.g., to see if they are allowed to read system properties or instantiate certain classes). Without supporting this native method, Duke JVM is effectively blocked from running standard ecosystem OSS JARs, significantly limiting its utility. Features are liabilities until they are used, and right now, our JVM can't use standard logging libraries.

📊 **Metric Definition:**
Success = The `slf4j_simple_smoke_runs_real_jar_bytecode` canary test in `oss_jar_smoke.rs` successfully passes the `System.getSecurityManager()` call without throwing an `Unsupported native` exception.

🔍 **Gap Analysis:**
- In standard JDKs (specifically Java 17+), `SecurityManager` is deprecated for removal (JEP 411) and by default, `System.getSecurityManager()` simply returns `null` unless explicitly enabled via command-line flags.
- Currently, Duke JVM has no native implementation for `System.getSecurityManager()Ljava/lang/SecurityManager;`, which causes an immediate halt during `<clinit>` of many common classes.

✅ **Acceptance Criteria:**
- The native method `java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;` must be implemented.
- The method must return `null` by default (slot reference of `None`), accurately mirroring the default modern JVM behavior.
- The execution must not panic or cause a JVM crash.
- Must handle the native method registration cleanly within the `duke-interpreter` standard library bridge.

🚫 **Out of Scope:**
- Full implementation of `SecurityManager` logic, policy file parsing, or access control contexts.
- Implementation of `System.setSecurityManager()`.
