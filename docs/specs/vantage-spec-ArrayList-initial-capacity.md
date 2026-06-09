# 🔭 Vantage: Spec for ArrayList.<init>(I)V

## 👤 User Story
"As a Java Developer or Library Maintainer (like SLF4J), I want to initialize an `ArrayList` with a specific initial capacity using the `ArrayList(int initialCapacity)` constructor, so that I can avoid dynamic resizing overhead when the approximate dataset size is known."

## ❓ The "So What?"
What business problem does this solve?
Currently, the Duke JVM implementation is blocked from running real-world libraries like `slf4j-simple` because it crashes when encountering the `java/util/ArrayList.<init>(I)V` constructor. Supporting this constructor is a critical step in achieving broader compatibility with the standard Java ecosystem and executing OSS JARs. It unblocks enterprise logging frameworks and removes an explicit blocker highlighted in our OSS test suite.

## 📈 Metric Definition
Success = The `slf4j-simple` OSS smoke test progresses past the `ArrayList.<init>(I)V` failure without triggering an "Unsupported native" error.
Success = `ArrayList.<init>(I)V` correctly initializes a size=0 list and handles negative inputs appropriately.

## 🔍 Gap Analysis
- **Current State:** The Duke interpreter implements native bindings for `ArrayList.<init>()V` (default constructor) and `ArrayList.<init>(Collection)V`. However, it lacks the explicit `(I)V` signature for initial capacity.
- **Market/Standard Lib:** The standard JVM (`java.util.ArrayList`) requires this constructor. While actual pre-allocation behavior is an implementation detail (Duke can dynamically size it anyway), the signature must be fulfilled to prevent `MethodNotFound` execution failures.
- **The Gap:** We need to provide a native implementation for `java/util/ArrayList.<init>(I)V` that acts identically to the no-args constructor for now, but respects the method signature.

## ✅ Acceptance Criteria
- Must register the native method `java/util/ArrayList.<init>(I)V` in the `stdlib` registry.
- The constructor must initialize an empty `ArrayList` (size `0`).
- If the `initialCapacity` is negative, it must throw a `java/lang/IllegalArgumentException` matching standard Java behavior.
- The change must not break any existing `ArrayList` benchmarks or tests.

## 🚫 Out of Scope
- Actually pre-allocating memory up-front (Duke's internal representation handles growth gracefully without strict capacity pre-allocation).
- Implementation details (Engineers will decide how to map this to `native_arraylist_init_with_capacity`).
