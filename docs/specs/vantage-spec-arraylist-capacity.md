# 🔭 Vantage: Spec for ArrayList.<init>(I)V

## 👤 User Story
"As a Software Engineer using Duke, I want `ArrayList` to support the initial capacity constructor `ArrayList.<init>(I)V`, so that I can pre-allocate lists to avoid unnecessary reallocations and run standard enterprise libraries like `slf4j-simple`."

## ❓ The "So What?"
What business problem does this solve?
Duke currently blocks on the `slf4j-simple` real-world JAR compatibility test because standard libraries use the `ArrayList(int initialCapacity)` constructor to optimize memory usage and CPU cycles when the target list size is known upfront. The current lack of this standard Java API constructor prevents the successful execution of common libraries like `slf4j` and heavily impacts real-world application adoption and performance tuning. Implementing it is essential for enterprise framework compatibility.

## 📈 Metric Definition
Success = The `slf4j-simple` smoke test (`oss_jar_smoke`) progresses past the `java/util/ArrayList.<init>(I)V` invocation without a "Unsupported native" missing error, maintaining Duke's stability.

## 🔍 Gap Analysis
- **Current State:** Duke JVM interpreter supports the default no-arg constructor `ArrayList()` and the `ArrayList(Collection c)` constructor. The single-int constructor `ArrayList(int)` is explicitly missing from the registered natives.
- **Market/Standard Lib:** Standard JVMs fully implement `java.util.ArrayList(int)` to initialize the internal backing array with the specified capacity, preventing immediate resizing operations on insertions.
- **The Gap:** We need to implement the native bridge `java/util/ArrayList.<init>(I)V` to correctly initialize the list state (e.g. size=0). Note that Duke's current `ArrayList` representation dynamically grows, so we mostly need to accept the capacity hint and initialize an empty list, but we must implement the method signature to satisfy the interpreter.

## ✅ Acceptance Criteria
- Must register the `java/util/ArrayList.<init>(I)V` native method in the Duke stdlib.
- Must correctly initialize the `ArrayList` object state (size = 0) upon invocation.
- Must execute the `slf4j-simple` test past the current ArrayList initialization blocker.

## 🚫 Out of Scope
- Actually pre-allocating memory for Duke's backing representation if the current architecture inherently grows the elements dynamically without fixed arrays. The key requirement is API signature compatibility to satisfy bytecodes.
- Rewriting the `ArrayList` native implementation to use standard Java arrays instead of Duke's dynamic fields.
