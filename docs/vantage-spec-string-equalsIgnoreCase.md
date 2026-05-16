# 🔭 Vantage: Spec for `java.lang.String.equalsIgnoreCase` Implementation

## 👤 User Story
"As a Java Framework Developer running libraries like SLF4J on Duke, I want the VM to natively support `java.lang.String.equalsIgnoreCase`, so that case-insensitive string comparisons function correctly without missing native method errors."

## ❓ The "So What?"
What business problem does this solve?
String manipulation is fundamental in almost any language. `equalsIgnoreCase` is frequently used for configuration parsing, HTTP header checking, and file extensions. Currently, Duke is blocked on running real-world code like `slf4j-simple` because this basic native method is missing. Without it, standard Java libraries crash before they even finish their initialization sequences. Implementing it unlocks the next milestone in OSS JAR compatibility, proving that Duke can handle basic string operations.

## 📈 Metric Definition
Success = `tests/oss_jar_smoke.rs` successfully progresses past the `String.equalsIgnoreCase` missing capability blocker during the execution of `slf4j-simple`.

## 🔍 Gap Analysis
- **Current State:** Duke lacks the native bridge for `java.lang.String.equalsIgnoreCase`.
- **Market/Standard Lib:** The standard JVM provides this either natively or via intrinsic functions to do fast, case-insensitive comparisons of char/byte arrays.
- **The Gap:** We need to implement the native bridge logic in `duke-interpreter` that allows Java's `String.equalsIgnoreCase` to properly call into Rust code, compare the underlying string data, and return a boolean result.

## ✅ Acceptance Criteria
- Must implement the native bridge required for `java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z`.
- Must handle `null` arguments correctly (returns `false` according to Java spec).
- Must correctly compare two Java Strings, returning `true` if they represent the same sequence of characters ignoring case differences, and `false` otherwise.
- Must extract the underlying string data correctly from both the `this` string and the argument string.

## 🚫 Out of Scope
- Full locale-dependent case folding (e.g. Turkish `i` vs `I`). Focus on standard ASCII case insensitivity or standard Unicode simple case folding as appropriate for an initial implementation.
