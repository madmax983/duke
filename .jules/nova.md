## 2024-05-24 - JAR Security Audit
**The Spark:** "I noticed we calculate complexities and basic graph analysis, but what if we can identify problematic usages in terms of security or stability? For instance, what if we want to find potentially dangerous API calls like `System.exit()`, `Runtime.exec()`, or Reflection APIs?"
**The Feature:** Implemented `audit.rs` and the `duke audit` command to scan a JAR file for invocations of dangerous JVM methods.
**The Potential:** Could be used as a pre-commit hook or CI check to prevent the introduction of arbitrary code execution vectors or abrupt JVM shutdown calls.

## 2024-05-06 - Method Fingerprinting
💡 **The Spark:** I noticed we have `decode` for instructions, but no way to track duplicated or vendored code across an entire JAR.
🚀 **The Feature:** Implemented `dump_fingerprints`, which generates a structural hash of instructions (ignoring constant pool offsets) for every method to detect identical logic blocks regardless of obfuscation.
🔮 **The Potential:** Can easily detect copied/vendored utility classes, redundant logic, or even spot malware variants hiding in obfuscated classes.
⚠️ **Risk:** Low. Completely isolated behind `#[cfg(feature = "nova")]` in `duke/src/fingerprint.rs`.
