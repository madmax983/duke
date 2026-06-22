# 🔭 Vantage: Spec for `ArrayList(int initialCapacity)`

## 👤 User Story
"As a Java Developer running application frameworks (like SLF4J) on Duke, I want to initialize an `ArrayList` with a specific initial capacity using the `new ArrayList(int)` constructor, so that I can optimize memory allocations when the required list size is known in advance."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke lacks the implementation for the native JVM bridge `java/util/ArrayList.<init>(I)V`. This blocks the execution of many real-world Java libraries (such as SLF4J and standard Spring components) that pre-allocate lists to avoid the overhead of dynamic resizing. Real-world JARs assume this fundamental constructor is available. By supporting this, we unblock the `oss_jar_smoke` pipeline from progressing past SLF4J initialization, significantly expanding Duke's compatibility footprint and enabling broader adoption for production workloads.

## 📈 Metric Definition
Success = The `slf4j-simple-2.0.13.jar` smoke test successfully progresses past the `Unsupported native: java/util/ArrayList.<init>(I)V` failure, and a standard Java program can instantiate `new ArrayList(10)` without throwing a `NoSuchMethodError` or `UnsupportedOperationException`.

## 🔍 Gap Analysis
- **Current State:** Duke has unimplemented native support for `java/util/ArrayList.<init>(I)V`. The current `ArrayList` implementation may only support the no-arg constructor or lack explicit handling for the capacity argument at the native level.
- **Market/Standard Lib:** The standard Java `ArrayList` class provides a constructor that takes an `int initialCapacity` to pre-size the underlying array, avoiding reallocation costs during subsequent `add()` operations.
- **The Gap:** We need to implement the native method `java/util/ArrayList.<init>(I)V` in the interpreter's native method registry to handle pre-allocated list initialization.

## ✅ Acceptance Criteria
- Must implement the native method `java/util/ArrayList.<init>(I)V`.
- Must allocate the underlying structural representation for an `ArrayList` while respecting the requested initial capacity.
- Must throw `java/lang/IllegalArgumentException` if the provided `initialCapacity` is negative.
- The `oss_jar_smoke` tests (specifically the SLF4J smoke test) must progress beyond the `ArrayList.<init>(I)V` unsupported native error.

## 🚫 Out of Scope
- Implementing custom memory pooling or advanced heap optimizations for array allocation beyond fulfilling the basic standard library contract.
- Modifying other `ArrayList` methods (like `add`, `remove`) unless explicitly required to support the new constructor.
