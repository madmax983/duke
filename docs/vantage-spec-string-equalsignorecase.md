# 🔭 Vantage: Spec for `java.lang.String.equalsIgnoreCase` Implementation

## 👤 User Story
"As a Java Developer running frameworks on Duke, I want the VM to natively support `java.lang.String.equalsIgnoreCase`, so that utility methods in core libraries and frameworks (like SLF4J) can compare strings case-insensitively without crashing the runtime due to missing natives."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke is blocked from advancing in real-world application compatibility (specifically the `slf4j-simple` oss-jar smoke test) because it throws an `Unsupported native` error when it encounters `java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z`. This is a foundational method in the Java standard library used extensively for configuration parsing, HTTP header comparison, and general string manipulation. Without it, even basic logging setups fail to initialize. Implementing this native bridge is a direct, required step to unblock the `slf4j` integration milestone and move Duke closer to running real-world enterprise code.

## 📈 Metric Definition
Success = The `tests/fixtures/oss-jars/slf4j-simple-2.0.13/oss_jar_smoke.rs` tests (or equivalent) progress past the `java/lang/String.equalsIgnoreCase` exception and either pass or fail on a subsequent, different missing capability. Additionally, explicit unit tests for `equalsIgnoreCase` handle both matching and non-matching case-insensitive pairs, as well as `null` arguments, correctly.

## 🔍 Gap Analysis
- **Current State:** The native method `java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z` is not implemented in Duke's `native.rs` or `stdlib.rs`.
- **Market/Standard Lib:** The OpenJDK standard library relies on this native method (or intrinsics) to perform fast, case-insensitive string comparisons.
- **The Gap:** We need to implement the Rust equivalent of case-insensitive string comparison and bridge it to the JVM's `String.equalsIgnoreCase` signature, properly handling null references and Java's internal string representation (often UTF-16 or Latin-1).

## ✅ Acceptance Criteria
- Must implement the native method `java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z`.
- Must return `true` if the strings are identical in length and their characters match regardless of case.
- Must return `false` if the strings differ in length or character content when ignoring case.
- Must return `false` if the argument passed is `null`.
- Must successfully unblock the SLF4J smoke test from its current failure state.

## 🚫 Out of Scope
- Locale-dependent collation or complex Unicode casing rules beyond standard ASCII/basic Unicode case folding required by the Java specification for `equalsIgnoreCase`.
- Full intrinsification/JIT compilation of the method; a standard native call bridge is sufficient.
