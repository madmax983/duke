# 🔭 Vantage: Spec for String.equalsIgnoreCase

## 👤 User Story
"As a Java Library Developer running on Duke, I want `String.equalsIgnoreCase` to be implemented, so that my applications can perform case-insensitive string comparisons, which is heavily used in configuration parsing, HTTP headers, and standard libraries like SLF4J."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke lacks support for `String.equalsIgnoreCase`. This is a foundational method in the Java standard library, often used for parsing user inputs, properties, and protocol headers. Without it, essential libraries like SLF4J fail during initialization when comparing configuration keys or log levels. Implementing this unblocks SLF4J and allows Duke to run a wider variety of real-world, third-party libraries.

## 📈 Metric Definition
Success = The `String.equalsIgnoreCase` native method correctly compares two strings ignoring case differences. The SLF4J simple smoke test progresses past this explicit missing capability.

## 🔍 Gap Analysis
- **Current State:** Duke fails to execute `String.equalsIgnoreCase`, throwing an `Unsupported native` error.
- **Market/Standard Lib:** Standard JVMs provide this method for case-insensitive comparison.
- **The Gap:** We need to implement a native bridge for `java.lang.String.equalsIgnoreCase(Ljava/lang/String;)Z` in Duke.

## ✅ Acceptance Criteria
- Must implement the native method `java.lang.String.equalsIgnoreCase(Ljava/lang/String;)Z`.
- Must return `true` if the strings are identical or differ only by case.
- Must return `false` if the strings are of different lengths or differ by characters other than case.
- Must return `false` if the argument is `null`.
- The SLF4J smoke test must progress past the `String.equalsIgnoreCase` blocker.

## 🚫 Out of Scope
- Full locale-dependent case folding. Standard `equalsIgnoreCase` relies on basic character lowercase/uppercase matching.
- Complex collation rules.
