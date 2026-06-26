# 🔭 Vantage: Spec for ArrayList Capacity Constructor

## 👤 User Story
"As a Java Developer running my code on Duke, I want the JVM to support the `java.util.ArrayList(int)` constructor (`<init>(I)V`), so that my applications and dependencies (like `slf4j-simple`) can pre-allocate lists with a specific initial capacity and execute without missing method errors."

## ❓ The "So What?"
What business problem does this solve?
Currently, third-party libraries like `slf4j-simple` are blocked from running on Duke because it lacks support for the standard `java/util/ArrayList.<init>(I)V` constructor. This prevents any serious real-world application from executing. By adding this basic collection capability, we unblock the `slf4j-simple` smoke test and move one step closer to full Java SE compatibility, which is critical for user adoption.

## 📈 Metric Definition
Success = The `slf4j-simple` smoke test successfully progresses past `java/util/ArrayList.<init>(I)V` during initialization, moving the CI canary further down the execution path.

## 🔍 Gap Analysis
- **Current State:** Duke fails when third-party bytecode invokes `java.util.ArrayList(int)` because the synthetic class or method is missing.
- **Market/Standard Lib:** The standard JVM's `ArrayList` provides a capacity constructor to optimize memory allocation by pre-sizing the backing array.
- **The Gap:** We need to provide a minimal, functional implementation of `java.util.ArrayList(int)` that satisfies the standard library expectations of third-party JARs.

## ✅ Acceptance Criteria
- Must support the `java.util.ArrayList` constructor taking an `int` parameter (`<init>(I)V`).
- Must correctly initialize the underlying storage with the specified capacity (or behave appropriately if it's a stub, as long as it satisfies the bytecode).
- Must throw `java.lang.IllegalArgumentException` if the provided capacity is negative, matching standard Java behavior.
- Must unblock the `slf4j-simple` smoke test from its current failure point.

## 🚫 Out of Scope
- Full implementation of all `ArrayList` methods (only build what is required to pass the current blocker and basic list operations).
- Other Java collections (e.g., `LinkedList`, `HashMap`) unless explicitly needed by the smoke test.
- Advanced `ArrayList` optimizations (e.g., SIMD array copying).
