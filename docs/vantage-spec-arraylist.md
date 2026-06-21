# 🔭 Vantage: Spec for `java.util.ArrayList` Implementation

## 👤 User Story
"As a Java Developer running my code on Duke, I want the VM to support `java.util.ArrayList` and its requisite native methods (like `System.arraycopy`), so that I can use standard dynamic arrays without my application crashing during initialization."

## ❓ The "So What?"
What business problem does this solve?
`ArrayList` is arguably the most ubiquitous data structure in Java. Nearly every real-world Java application and third-party library, including logging frameworks like `slf4j`, relies on it heavily for dynamic collections. Currently, Duke's OSS JAR smoke harness is blocked from executing `slf4j-simple` because it fails to instantiate `java.util.ArrayList` (specifically `java/util/ArrayList.<init>(I)V`). If Duke cannot run `ArrayList`, it cannot run 99% of Java software, rendering the VM a toy rather than a useful product. Implementing this allows us to move past the current blocker and support real-world ecosystem libraries.

## 📈 Metric Definition
Success = The `slf4j-simple` OSS smoke test successfully progresses past `java/util/ArrayList.<init>(I)V`, and a basic `ArrayList` fixture test can `add()`, `get()`, and iterate over elements without panicking.

## 🔍 Gap Analysis
- **Current State:** Duke's execution engine fails when attempting to initialize `java.util.ArrayList` with an initial capacity. This is blocking the `slf4j-simple` OSS JAR smoke test.
- **Market/Standard Lib:** The standard JVM fully supports `java.util.ArrayList`, executing its underlying array resizing via optimized native methods.
- **The Gap:** We need to ensure that whatever JVM intrinsics, native bridges, or synthetic classes are required by the OpenJDK `ArrayList` implementation (e.g. `System.arraycopy()`, object array instantiation) are fully implemented in Duke.

## ✅ Acceptance Criteria
- Must support instantiating `java.util.ArrayList` with and without an initial capacity parameter (`<init>(I)V`).
- Must support underlying native dependencies required by the OpenJDK `ArrayList` implementation (e.g., `System.arraycopy` for Object arrays).
- Must successfully execute basic `ArrayList` operations (`add()`, `get()`) in a unit fixture.
- The `slf4j-simple` smoke test must progress beyond the `ArrayList` blocker.

## 🚫 Out of Scope
- Complete implementation of the entire Java Collections Framework (e.g., `LinkedList`, `HashMap`, `TreeMap`). Focus strictly on unblocking `ArrayList`.
- Highly optimized custom array resizing heuristics. Standard OpenJDK behavior is sufficient.
- Thread-safe dynamic array equivalents like `CopyOnWriteArrayList` (unless strictly required by the next step in `slf4j`).
