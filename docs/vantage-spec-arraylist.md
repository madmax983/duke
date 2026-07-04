# 🔭 Vantage: Spec for ArrayList

## 👤 User Story
"As a Java Developer running my code on Duke, I want the VM to natively support or correctly interpret `java.util.ArrayList`, so that I can use standard dynamic arrays without my application crashing or being blocked at initialization."

## ❓ The "So What?"
What business problem does this solve?
`java.util.ArrayList` is one of the most ubiquitous data structures in the Java ecosystem. Currently, running real-world third-party JARs, such as `slf4j-simple`, is completely blocked because Duke fails to handle the `ArrayList` constructor (`java/util/ArrayList.<init>(I)V`). Supporting this foundational collection is absolutely necessary to run standard Java libraries and frameworks, vastly increasing the utility of the VM.

## 📈 Metric Definition
Success = The `slf4j-simple` OSS smoke test progresses past the `java/util/ArrayList.<init>(I)V` blocker, and basic `ArrayList` operations (initialization, `add`, `get`, `size`) execute successfully.

## 🔍 Gap Analysis
- **Current State:** Duke currently fails when hitting `java/util/ArrayList.<init>(I)V`, as tracked in issue #854.
- **Market/Standard Lib:** The standard JVM supports `ArrayList` through bytecode backed by dynamically resized `Object[]` arrays, and potentially native optimizations.
- **The Gap:** We need to implement the necessary bytecode execution support, native method bridges, or standard library injection in `crates/duke-interpreter/src/native.rs` to allow `ArrayList` to initialize and perform basic operations.

## ✅ Acceptance Criteria
- Must support the `ArrayList(int initialCapacity)` constructor (`java/util/ArrayList.<init>(I)V`).
- Must support adding elements to the list.
- Must support retrieving elements by index.
- Must support retrieving the size of the list.
- The `slf4j-simple` OSS smoke test must no longer be blocked by `ArrayList` initialization.

## 🚫 Out of Scope
- Full implementation of the entire Java Collections Framework (e.g., `LinkedList`, `HashMap` internals, unless explicitly required).
- Advanced Java 8+ stream or spliterator optimizations for `ArrayList`.
