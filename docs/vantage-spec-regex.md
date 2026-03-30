# 🔭 Vantage: Spec for Regular Expressions (`java.util.regex`)

## 👤 User Story
"As a Data Engineer running my code on Duke, I want to use standard Java regular expressions to parse and validate text, so that I can process string data efficiently without writing manual string manipulation loops."

## ❓ The "So What?"
What business problem does this solve?
String manipulation is at the core of almost all business logic, from validating email addresses to parsing log files. Java's `java.util.regex` package is the standard tool for this. Without it, developers are forced to use primitive and error-prone `String.indexOf` or `substring` methods. Providing native regex support unblocks a massive category of text-processing workloads, modernizes the string manipulation toolkit, and paves the way for advanced data extraction pipelines on Duke.

## 📈 Metric Definition
Success = A Java program running on Duke can successfully execute `Pattern.compile("[a-z]+").matcher("duke").matches()` and return `true`, and standard string methods like `String.replaceAll()` and `String.split()` function correctly using standard regular expressions.

## 🔍 Gap Analysis
- **Current State:** Duke has basic string operations but lacks support for `java.util.regex.Pattern` and `Matcher`.
- **Market/Standard Lib:** The standard JDK implements regex using a complex internal state machine. Developers expect regex to "just work" with reasonable performance.
- **The Gap:** We lack the execution capability for regex operations. Engineering will need to evaluate whether to interpret the massive Java standard library regex implementation directly or to build native bridges to a fast host-level regex engine.

## ✅ Acceptance Criteria
- Must support compiling standard regular expressions via `java.util.regex.Pattern`.
- Must support matching text, finding substrings, and extracting capture groups via `java.util.regex.Matcher`.
- Must support `String.split` and `String.replaceAll` using regex patterns.
- Must correctly raise `java.util.regex.PatternSyntaxException` for invalid regular expressions.

## 🚫 Out of Scope
- 100% strict compliance with highly obscure Java-specific regex edge cases (e.g., complex look-behind assertions if a native engine is used).
- Performance parity with HotSpot's compiled regex execution (functional correctness is the primary goal for Phase 1).
