# 🔭 Vantage: Spec for `System.getSecurityManager()`

## 👤 User Story
"As a JVM Developer looking to run standard third-party libraries (like slf4j-simple) on Duke, I want the VM to provide a minimal implementation of `System.getSecurityManager()`, so that these libraries can initialize without failing on unsupported native methods."

## ❓ The "So What?"
What business problem does this solve?
Currently, our `slf4j-simple` compatibility smoke test is blocked and ignored because Duke lacks an implementation for `java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;`. Many enterprise and legacy libraries check for the presence of a security manager as part of their standard initialization routine. Without a functional (even if stubbed/null-returning) implementation, Duke cannot execute a large swath of standard Java libraries out of the box, severely limiting its utility and real-world compatibility. Implementing this unblocks OSS testing.

## 📈 Metric Definition
Success = The ignored `slf4j_simple_smoke_runs_real_jar_bytecode` test in `oss_jar_smoke.rs` can be un-ignored and passes successfully.

## 🔍 Gap Analysis
- **Current State:** Duke's standard library native implementations in `stdlib.rs` and `native.rs` do not register or handle the `System.getSecurityManager()` native call.
- **Market/Standard Lib:** Modern standard JVMs support this method (though deprecated in newer versions). For many environments, returning `null` implies no security manager is installed, which allows the application to proceed normally.
- **The Gap:** We need to implement a native handler for `java/lang/System.getSecurityManager()Ljava/lang/SecurityManager;` that simply returns `null` (or a valid SecurityManager object if implemented later), and register it in `stdlib.rs`.

## ✅ Acceptance Criteria
- Must implement `java.lang.System.getSecurityManager()` native handler.
- Must return `null` to indicate no security manager is active, satisfying standard initialization paths.
- Must register the new native handler correctly in the Duke interpreter registry.
- Must unblock and pass the `slf4j_simple_smoke_runs_real_jar_bytecode` smoke test.

## 🚫 Out of Scope
- Actually implementing a functional `java.lang.SecurityManager` and the associated security check policies.
- Supporting legacy Applet or RMI security managers.
