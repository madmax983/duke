## 2026-05-06 - Bytecode Diff

**The Spark:** "I noticed we parse and decode bytecodes, and we have multiple analysis tools, but no way to easily compare two versions of a class file to see what changed."
**The Feature:** "Implemented `duke diff` to compare class files."
**The Potential:** "Could be used for reverse-engineering patches, understanding compiler optimizations, and tracking down regressions."
**Risk:** "Low. Isolated in `duke/src/diff.rs`."
