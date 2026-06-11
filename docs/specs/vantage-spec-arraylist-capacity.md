# 🔭 Vantage: Spec for ArrayList Initial Capacity Constructor

## 👤 User Story
"As a Java Developer running third-party libraries on Duke, I want the VM to natively support initializing an `ArrayList` with a specific initial capacity (`ArrayList.<init>(I)V`), so that common ecosystem frameworks can allocate memory efficiently without triggering an unsupported native exception."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke's interpreter halts execution when it encounters the `ArrayList(int initialCapacity)` constructor because the native backing for it is unimplemented. This specific signature is the current explicit blocker preventing the execution of `slf4j-simple`, a foundational logging framework in the Java ecosystem. Without it, developers cannot run basic off-the-shelf software. Implementing this unlocks progress on the real-world OSS JAR compatibility metrics, specifically pushing the `slf4j` canary test further, which is critical for demonstrating Duke's viability as a production JVM.

## 📈 Metric Definition
Success = The `slf4j_simple_smoke_runs_real_jar_bytecode` test suite progresses past the `java/util/ArrayList.<init>(I)V` invocation without panicking or throwing an `Unsupported native` exception. A user should be able to instantiate an `ArrayList` providing an integer capacity.

## 🔍 Gap Analysis
- **Current State:** Duke provides basic support for `ArrayList.<init>()V` (default constructor) and `ArrayList.<init>(Collection)V`. However, it lacks the explicit initial-capacity constructor `ArrayList.<init>(I)V`.
- **Market/Standard Lib:** The standard Java `ArrayList` provides this constructor as a performance optimization, allowing users to pre-allocate an internal array of the correct size to prevent expensive reallocation during bulk insertions. Standard logging libraries, networking tools, and basic data structures use this heavily.
- **The Gap:** We need a corresponding native bridge registered in the interpreter to accept the `int` argument, validate it, and initialize the internal list representation appropriately.

## ✅ Acceptance Criteria
- Must register a handler for `java/util/ArrayList.<init>(I)V`.
- Must accept a single integer argument (the initial capacity) alongside the object reference.
- Must not panic or fail if the argument is provided.
- Must treat the capacity as a hint or correctly allocate internal resources, maintaining compatibility with the existing internal `ArrayList` data structure implementation.
- Must reject negative capacity arguments by throwing a `java/lang/IllegalArgumentException`, matching standard JVM behavior.

## 🚫 Out of Scope
- Implementing full bounds checking logic beyond negative capacities (that's an Engineering implementation detail).
- Expanding the `ArrayList` API surface with other methods (`addAll`, `removeIf`, etc.) outside of this specific constructor.
- Re-architecting how `ArrayList` memory is managed under the hood.
