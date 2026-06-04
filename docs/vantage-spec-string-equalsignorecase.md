# 🔭 Vantage: Spec for String.equalsIgnoreCase Native Support

## 👤 User Story
"As a Java Developer running libraries like SLF4J on Duke, I want the VM to natively support `String.equalsIgnoreCase`, so that my applications can perform string comparisons and initialization without crashing."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke lacks native support for `java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z`. This prevents essential libraries like SLF4J from initializing, blocking our OSS JAR compatibility milestone (specifically the `slf4j-simple` smoke test). Implementing this native method unblocks the execution of real-world libraries and progresses our compatibility goals.

## 📈 Metric Definition
Success = The SLF4J smoke test progresses past the `Unsupported native: java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z` missing capability error.

## 🔍 Gap Analysis
- **Current State:** The SLF4J smoke test is blocked because Duke throws an "Unsupported native: java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z" error in `tests/oss_jar_smoke.rs`.
- **Market/Standard Lib:** The standard JVM implements `String.equalsIgnoreCase`.
- **The Gap:** We need to implement the native bridge for `java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z` in Duke.

## ✅ Acceptance Criteria
- Must implement the native method `java/lang/String.equalsIgnoreCase(Ljava/lang/String;)Z`.

## 🚫 Out of Scope
- Implementing other missing `String` native methods.
