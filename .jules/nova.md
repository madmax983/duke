## 2024-05-02 - Bytecode Diff

💡 **The Spark:** "We can load and parse JARs, but we can't tell what changed between two versions of a library. Can we diff them at the bytecode level?"
🚀 **The Feature:** "Implemented `jar-diff` CLI command to compare the bytecode hashes of methods between two JAR files."
🔮 **The Potential:** "Could be used for vulnerability patch analysis or library version diffing."
⚠️ **Risk:** "Low. Isolated in `src/jar_diff.rs` and safely gated behind the `nova` feature flag."

## 2024-05-18 - Dominator Tree Analysis

💡 **The Spark:** "I noticed we decode `LocalVariableTable` in `duke-classfile` but don't do much interesting structural analysis with it alongside basic blocks. We have reachability and basic blocks; let's compute Dominator Trees!"
🚀 **The Feature:** "Implemented `dominators.rs` module providing control flow graph dominator and immediate dominator set algorithms."
🔮 **The Potential:** "Could be used for SSA construction, loop detection, and advanced optimization passes."
⚠️ **Risk:** "Low. Isolated in `crates/duke-bytecode/src/dominators.rs` and safely gated behind the `nova` feature flag."
