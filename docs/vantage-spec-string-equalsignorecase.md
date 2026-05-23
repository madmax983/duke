# 🔭 Vantage: Spec for String.equalsIgnoreCase

## 👤 User Story
"As a Java Framework Developer, I want the VM to support `java.lang.String.equalsIgnoreCase`, so that my applications can perform case-insensitive string comparisons correctly."

## ❓ The "So What?"
What business problem does this solve?
Case-insensitive string comparison is a fundamental operation used in parsing configurations, HTTP headers, and general text processing. Currently, the `slf4j-simple` OSS smoke test is completely blocked because Duke lacks the native implementation for `java.lang.String.equalsIgnoreCase`. By unblocking this, we take a necessary step towards running standard real-world Java applications, increasing Duke's utility.

## 📈 Metric Definition
Success = The `slf4j-simple` smoke test progresses past the `String.equalsIgnoreCase` missing capability error.

## 🔍 Gap Analysis
- **Current State:** Duke throws an `Error::MethodNotFound` when `java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z` is invoked.
- **Market/Standard Lib:** The standard JVM natively implements case-insensitive string equality to quickly compare text.
- **The Gap:** The native bridge for this specific method is missing in `duke-interpreter`'s standard library bindings.

## ✅ Acceptance Criteria
- Must implement the native bridge for `java.lang.String.equalsIgnoreCase(Ljava/lang/String;)Z`.
- Must handle `null` references gracefully (returning `false`).
- Must return `true` if the two strings are identical ignoring case considerations.
- Must return `false` if the strings are different lengths or have different characters.

## 🚫 Out of Scope
- Full locale-dependent case folding.
- Implementing other missing `String` native methods.
