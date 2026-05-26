# 🔭 Vantage: Spec for String.equalsIgnoreCase

## 👤 User Story
As a Developer running third-party JARs (like SLF4J), I want `java/lang/String.equalsIgnoreCase` to be supported so that my application can initialize and execute string comparisons without crashing due to unsupported native methods.

## 💼 Business Problem ("So What?")
Our Duke JVM's value is derived from its ability to run real-world software. Currently, the SLF4J smoke test is blocked because Duke lacks an implementation for `String.equalsIgnoreCase(String)Z`. Adding this basic standard library capability allows us to unblock the next phase of compatibility testing and moves Duke closer to utility, reducing the gap between an experimental project and a useful execution engine. Complexity is a cost, utility is a revenue—if we cannot even compare strings, we have zero utility for most modern libraries.

## 📊 Success Metrics
- **Success:** The `oss_jar_smoke` test progresses completely past `java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z`.
- **Performance:** Method invocation latency should be negligible (ideally comparable to standard string comparison).

## 🔍 Gap Analysis
- **Standard Libs / Market:** Standard Java SE requires case-insensitive string comparison for many core APIs (like HTTP headers parsing, case-insensitive mapping, classloading paths, etc.).
- **Current State:** Duke fails fast when this is invoked via SLF4J, completely halting progress. We have `java/lang/String.equals` mapped, but not `equalsIgnoreCase`.

## ✅ Acceptance Criteria
- Must define a native method for `java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z`.
- Must handle `null` arguments gracefully without panicking (should evaluate to `false` if `anotherString` is null, or `true` if both references are identical).
- Must perform a case-insensitive string comparison.
- Must execute SLF4J test past this blocker.

## 🚫 Out of Scope
- Full locale-dependent Unicode case-folding algorithms (a simple ASCII or standard locale-agnostic case-insensitive comparison is sufficient for MVP).
- Implementing other unsupported `String` native methods.
