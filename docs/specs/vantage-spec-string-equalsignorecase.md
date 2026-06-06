# 🔭 Vantage: Spec for `String.equalsIgnoreCase`

## 👤 User Story
"As a Java Application Developer running my code on Duke, I want the VM to natively support `java/lang/String.equalsIgnoreCase`, so that my applications and libraries can reliably perform case-insensitive string comparisons, such as parsing configuration properties, without crashing."

## ❓ The "So What?"
What business problem does this solve?
Many foundational Java libraries and frameworks heavily rely on `String.equalsIgnoreCase` for basic string processing, such as parsing HTTP headers, environment variables, and configuration files. Most notably, the popular SLF4J logging framework uses this method during its early static initialization phase. Currently, Duke's inability to execute this native method entirely blocks the SLF4J smoke test and prevents enterprise-grade applications from starting. By implementing this method, we unblock a massive swath of standard library usage and allow real-world third-party code to run.

## 📈 Metric Definition
Success = The `slf4j_simple_smoke_runs_real_jar_bytecode` integration test progresses past the "Unsupported native: java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z" error, and a Java application can successfully execute `"Hello".equalsIgnoreCase("hello")` and receive `true`.

## 🔍 Gap Analysis
- **Current State:** Duke throws an `Unsupported native` error when executing code that invokes `java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z`. This explicitly blocks the SLF4J initialization sequence in our OSS JAR smoke test.
- **Market/Standard Lib:** The standard JVM implementation provides a fast, robust implementation of `String.equalsIgnoreCase` that compares the characters of two string objects, ignoring differences in case.
- **The Gap:** We need to provide a native method bridge in Duke (`duke-interpreter/src/native.rs` and `duke-interpreter/src/stdlib.rs`) to properly handle the execution of `java/lang/String.equalsIgnoreCase`, comparing the internal byte/char arrays of Duke's String representation.

## ✅ Acceptance Criteria
- Must implement the `equalsIgnoreCase` native method bridge for `java/lang/String` in Duke's standard library.
- Must correctly return `true` if the two strings have the same length and corresponding characters are equal ignoring case.
- Must correctly return `false` if the strings differ in length or characters.
- Must correctly return `false` if the argument passed is `null`.
- Must properly interact with Duke's internal representation of Java `String` objects (e.g., retrieving the underlying value byte array and encoding).

## 🚫 Out of Scope
- Locale-dependent case mapping or full Unicode case-folding normalization beyond what is strictly specified by the standard `equalsIgnoreCase` contract.
- Optimizing this comparison via JIT compiler intrinsics. We only need the basic interpreter native implementation to function correctly.
