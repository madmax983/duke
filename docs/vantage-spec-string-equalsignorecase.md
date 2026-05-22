# 🔭 Vantage: Spec for java.lang.String.equalsIgnoreCase Support

## 👤 User Story
"As a Java Application Developer running on Duke, I want the VM to natively support `java.lang.String.equalsIgnoreCase`, so that my applications and dependencies can parse configurations and standard formats seamlessly without crashing."

## ❓ The "So What?"
What business problem does this solve?
Many enterprise libraries, including the SLF4J logging framework, rely on case-insensitive string comparisons for tasks like parsing log levels or headers (e.g., matching "INFO" vs "info"). Currently, Duke blocks execution when encountering `String.equalsIgnoreCase`. Without this utility, Duke cannot host these ubiquitous libraries, blocking real-world adoption. Fixing this unlocks the next stage of our OSS JAR compatibility roadmap.

## 📈 Metric Definition
Success = The Duke JVM can execute `String.equalsIgnoreCase` successfully, and the SLF4J smoke test progresses past the current `Unsupported native: java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z` blocker.

## 🔍 Gap Analysis
- **Current State:** Duke lacks the execution support for `java.lang.String.equalsIgnoreCase`.
- **Market/Standard Lib:** The standard JVM provides this functionality to allow case-ignoring equality checks on standard Strings.
- **The Gap:** We need an implementation for `java.lang.String.equalsIgnoreCase` that can evaluate case-insensitive equality of two String objects within the VM.

## ✅ Acceptance Criteria
- Must return `true` if the two strings are of the same length and corresponding characters are equal ignoring case.
- Must return `false` if the strings are of different lengths.
- Must return `false` if the provided argument is `null`.
- Must return `true` if the provided argument is the exact same reference.

## 🚫 Out of Scope
- Full locale-dependent Unicode case folding (basic Java definition is sufficient).
- Implementing `compareToIgnoreCase` or other `String` case methods.
