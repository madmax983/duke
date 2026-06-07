# 🔭 Vantage: Spec for String.equalsIgnoreCase

## 👤 User Story
"As a Developer running text-processing applications on Duke, I want the VM to support `String.equalsIgnoreCase`, so that I can perform case-insensitive string comparisons without allocating new uppercase/lowercase String instances."

## ❓ The "So What?"
What business problem does this solve?
`String.equalsIgnoreCase` is a foundational Java API heavily used by standard libraries (like slf4j) for configuration parsing, HTTP header checking, and basic text processing. Without it, even basic logging libraries fail to initialize and execute. Supporting this enables a vast swath of standard libraries to function correctly on Duke, expanding its compatibility and utility for real-world enterprise applications.

## 📈 Metric Definition
Success = The `slf4j-simple` OSS JAR smoke test advances past the `String.equalsIgnoreCase` missing native method blocker.

## 🔍 Gap Analysis
- **Current State:** Duke has basic `String` operations but `native_string_equals_ignore_case` is missing from the `duke-interpreter` natives.
- **Market/Standard Lib:** Standard JVMs (HotSpot) support fast, optimized case-insensitive string comparison natively.
- **The Gap:** We need to implement the JVM native bridge for `java.lang.String.equalsIgnoreCase(Ljava/lang/String;)Z` to handle the string comparison logic.

## ✅ Acceptance Criteria
- Must implement the native method for `String.equalsIgnoreCase`.
- Must correctly compare strings ignoring case.
- Must handle `null` arguments gracefully by returning `false`.

## 🚫 Out of Scope
- Full Unicode locale-dependent folding (only standard Java `equalsIgnoreCase` rules are required).
