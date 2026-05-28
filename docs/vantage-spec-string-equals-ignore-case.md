# 🔭 Vantage: Spec for String.equalsIgnoreCase

## 👤 User Story
"As a Library Developer running my code on Duke, I want `java.lang.String.equalsIgnoreCase` to be supported natively, so that my configuration parsing and protocol logic can compare strings robustly without being broken by case sensitivity."

## ❓ The "So What?"
What business problem does this solve?
Case-insensitive string comparison is a fundamental building block for many Java libraries. It is heavily used in configuration parsing (e.g. interpreting "True", "true", or "TRUE" interchangeably), handling HTTP headers, and interpreting environment variables. Without `equalsIgnoreCase`, any library relying on this method (like the SLF4J simple logger which parses `simplelogger.properties`) fails to execute, hard-blocking the system and stopping the VM from running standard Java applications. Supporting it expands Duke's compatibility footprint and removes a key blocker in real-world use cases.

## 📈 Metric Definition
Success = The `java.lang.String.equalsIgnoreCase(Ljava/lang/String;)Z` native method is successfully implemented. The `slf4j_simple_smoke_runs_real_jar_bytecode` integration test can bypass this missing capability and continue executing.

## 🔍 Gap Analysis
- **Current State:** Duke fails to execute Java code that calls `String.equalsIgnoreCase` because it lacks the native method implementation. This is actively blocking the execution of `slf4j-simple`.
- **Market/Standard Lib:** The standard JVM `String` class provides this capability natively (or efficiently delegates to standard core libraries) by comparing character content without case sensitivity.
- **The Gap:** We need to provide a native implementation for `java/lang/String.equalsIgnoreCase` that pulls the string data from Duke's heap, performs a case-insensitive comparison, and returns a boolean value.

## ✅ Acceptance Criteria
- Must natively implement `java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z`.
- Must return `true` if the strings have the same length and matching characters regardless of case (e.g., ASCII uppercase vs lowercase).
- Must return `false` if the lengths differ or characters do not match.
- Must return `false` if the provided argument is `null` (or return `true` if both the receiver and argument are identically null/empty in Java semantics, though Java prevents null receivers).
- The implementation must allow the `slf4j-simple` smoke test to progress past the current `Unsupported native: java/lang/String.equalsIgnoreCase` error.

## 🚫 Out of Scope
- Full Unicode case-folding completeness (e.g., Turkish 'I' or complex ligature rules) if basic ASCII/Latin-1 case-insensitivity covers the immediate needs for simple configurations. We only need the basic functional parity for the tests.
