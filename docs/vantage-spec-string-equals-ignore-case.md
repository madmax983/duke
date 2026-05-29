# 🔭 Vantage: Spec for `java.lang.String.equalsIgnoreCase` Implementation

## 👤 User Story
"As a Java Application Developer utilizing third-party libraries like SLF4J, I want the VM to support `java.lang.String.equalsIgnoreCase`, so that my applications can perform case-insensitive string comparisons essential for parsing configuration, headers, and environment variables."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke fails to execute the SLF4J smoke test because it encounters an unsupported native method: `String.equalsIgnoreCase`. Case-insensitive string comparison is a foundational utility used pervasively across standard Java libraries and frameworks (e.g., parsing "true"/"false" properties, matching protocol headers). Without this capability, robust real-world libraries fail during their static initialization or configuration phases, blocking Duke from running standard ecosystem software.

## 📈 Metric Definition
Success = Duke successfully executes `String.equalsIgnoreCase` operations and the `slf4j-simple` OSS smoke test progresses past the current `String.equalsIgnoreCase` capability blocker.

## 🔍 Gap Analysis
- **Current State:** Duke lacks the native bridge or synthetic implementation for `java.lang.String.equalsIgnoreCase(String)`.
- **Market/Standard Lib:** The standard JVM `String` class provides this method to handle case-insensitive string equality checks, which takes locale-independent or simple case folding into account.
- **The Gap:** We need to implement the corresponding native handler in `duke-interpreter` for `equalsIgnoreCase` so that it compares two Java strings case-insensitively according to the JVM specification.

## ✅ Acceptance Criteria
- Must implement `java.lang.String.equalsIgnoreCase(String)`.
- Must return `true` if both strings are of the same length and corresponding characters are equal ignoring case.
- Must return `false` if the strings are of different lengths or have characters that do not match when ignoring case.
- Must gracefully handle `null` arguments (returning `false` as per standard Java behavior without throwing a `NullPointerException`).

## 🚫 Out of Scope
- Implementing full complex locale-dependent string comparison (`Collator` rules).
- Modifying or implementing other `String` comparison methods like `compareToIgnoreCase` unless strictly necessary for this task.
