## 2024-05-02 - Bytecode Diff

💡 **The Spark:** "We can load and parse JARs, but we can't tell what changed between two versions of a library. Can we diff them at the bytecode level?"
🚀 **The Feature:** "Implemented `jar-diff` CLI command to compare the bytecode hashes of methods between two JAR files."
🔮 **The Potential:** "Could be used for vulnerability patch analysis or library version diffing."
⚠️ **Risk:** "Low. Isolated in `src/jar_diff.rs` and safely gated behind the `nova` feature flag."
