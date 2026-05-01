## 2024-05-24 - JAR Security Audit
**The Spark:** "I noticed we calculate complexities and basic graph analysis, but what if we can identify problematic usages in terms of security or stability? For instance, what if we want to find potentially dangerous API calls like `System.exit()`, `Runtime.exec()`, or Reflection APIs?"
**The Feature:** Implemented `audit.rs` and the `duke audit` command to scan a JAR file for invocations of dangerous JVM methods.
**The Potential:** Could be used as a pre-commit hook or CI check to prevent the introduction of arbitrary code execution vectors or abrupt JVM shutdown calls.
