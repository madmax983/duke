# 🔭 Vantage: Spec for `ArrayList.<init>(I)V`

## 👤 User Story
"As a Java Application executing on Duke, I want to instantiate an `ArrayList` with a specific initial capacity using `ArrayList.<init>(I)V`, so that I can optimize memory allocations when I know the approximate number of elements in advance."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke lacks the native implementation for `java/util/ArrayList.<init>(I)V`. This constructor is a foundational piece of the Java Collections Framework, utilized heavily by standard libraries and third-party dependencies to pre-allocate backing arrays and avoid the cost of dynamic resizing. Most critically, its absence is the current blocker preventing `slf4j-simple` from running on Duke JVM, halting our progress in running real-world OSS JARs through our CI pipeline. Implementing this will unblock SLF4J and improve compatibility with modern Java workloads.

## 📈 Metric Definition
Success = The `slf4j_simple_smoke_runs_real_jar_bytecode` test suite progresses past the `ArrayList.<init>(I)V` invocation without encountering a missing native method error.

## 🔍 Gap Analysis
- **Current State:** Duke implements `ArrayList.<init>()V` (default constructor) and `ArrayList.<init>(Collection)V`, but `native_arraylist_init_capacity` (handling `I`) is missing from `crates/duke-interpreter/src/native.rs` and `crates/duke-interpreter/src/stdlib.rs`.
- **Market/Standard Lib:** Standard HotSpot/OpenJ9 JVMs initialize the backing `Object[]` array with the provided integer capacity to optimize contiguous memory allocation.
- **The Gap:** We need to provide a native method mapping for `java/util/ArrayList.<init>(I)V` that accepts the object reference and the `int` capacity slot, validating the capacity and initializing the list structure.

## ✅ Acceptance Criteria
- Must implement the native method `native_arraylist_init_capacity` matching the signature `(I)V`.
- Must set the `size` field (index 0) of the `ArrayList` instance to `0`.
- Must throw a `java.lang.IllegalArgumentException` if the provided integer capacity is less than 0.
- Must be registered in `stdlib.rs` under `java/util/ArrayList`.

## 🚫 Out of Scope
- Actually allocating a separate `Object[]` backing array. Duke uses a flattened inline memory model for `ArrayList` fields (e.g., `fields[0] = size`, `fields[1..] = elements`), so the `initialCapacity` hint does not change the memory layout; it can be treated as a no-op structural initialization beyond validation.
