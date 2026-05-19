# 🔭 Vantage: Spec for `java.lang.String.equalsIgnoreCase`

## 👤 User Story
"As a Java Developer running my code on Duke, I want the VM to natively support `java.lang.String.equalsIgnoreCase`, so that I can compare text without writing manual lowercasing logic or dealing with unsupported native errors."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke's OSS JAR smoke tests for `slf4j` (a ubiquitous logging framework in the Java ecosystem) are explicitly blocked on this missing native method. The `slf4j-simple` test halts with: `Unsupported native: java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z`.
Unblocking this method clears a critical roadblock in OSS library compatibility, pushing Duke closer to being a viable JVM for modern, real-world enterprise deployments. By implementing this specific string comparison, we remove friction for adopting any library that does simple header parsing, configuration file reading, or command-line argument validation.

## 📈 Metric Definition
Success = The `slf4j-simple_smoke_surfaces_next_missing_capability_explicitly` test in `tests/oss_jar_smoke.rs` progresses past the `java/lang/String.equalsIgnoreCase` failure and identifies the *next* missing capability, proving this native bridge is correctly invoked and returns a boolean without panicking.

## 🔍 Gap Analysis
- **Current State:** Duke throws an `Unsupported native` error when `java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z` is invoked because the native implementation is missing in `crates/duke-interpreter/src/native.rs` and `stdlib.rs`.
- **Market/Standard Lib:** The standard JVM provides this method to do a case-insensitive string comparison.
- **The Gap:** We need a Rust implementation in Duke's interpreter that intercepts this call, unwraps the Java String references, performs a case-insensitive comparison, and returns a boolean `Slot`.

## ✅ Acceptance Criteria
- Must natively implement `String.equalsIgnoreCase` accepting a `String` reference argument.
- Must return `Slot::Int(1)` for true and `Slot::Int(0)` for false.
- Must correctly handle the case where the argument is `null` (should return false).
- Must correctly handle identical strings, strings that only differ by case, and strings of different lengths.
- The `slf4j-simple` smoke test must progress to the next blocker.

## 🚫 Out of Scope
- Implementing full locale-aware Unicode case-folding. Basic ASCII case-folding (as per standard `String::eq_ignore_ascii_case` or similar basic Unicode case handling) is acceptable for this phase unless `slf4j` explicitly requires complex Unicode folding.
- Refactoring the entire `String` native implementation module.
