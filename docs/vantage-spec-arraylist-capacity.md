# 🔭 Vantage: Spec for ArrayList Capacity Constructor `ArrayList.<init>(I)V`

## 👤 User Story
"As a Java Application Developer running my code on Duke, I want the VM to support the `ArrayList(int initialCapacity)` constructor, so that my applications can pre-allocate lists to avoid unnecessary dynamic resizing and improve performance."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke supports the default `ArrayList.<init>()V` constructor and the collection-based `ArrayList.<init>(Collection)V` constructor. However, it lacks support for the capacity-based constructor `ArrayList.<init>(I)V`. Many real-world libraries and frameworks, including `slf4j-simple`, rely heavily on this constructor to optimize list creations. Our test `tests/oss_jar_smoke.rs` explicitly blocks on this missing native method during the execution of `slf4j-simple`. Without it, we cannot achieve broader OSS compatibility or execute widely-used Java libraries that utilize pre-allocated `ArrayList`s. Implementing this constructor is a direct unblocker for `slf4j-simple` compatibility.

## 📈 Metric Definition
Success = The `slf4j-simple` integration test (`oss_jar_smoke.rs`) progresses past the current `java/util/ArrayList.<init>(I)V` blocker, and users can successfully instantiate an `ArrayList` specifying an initial capacity.

## 🔍 Gap Analysis
- **Current State:** Duke has native implementations for `ArrayList.<init>()V` and `ArrayList.<init>(Collection)V` in `crates/duke-interpreter/src/native.rs` and `crates/duke-interpreter/src/stdlib.rs`.
- **Market/Standard Lib:** The standard JVM `java.util.ArrayList` provides a constructor `ArrayList(int initialCapacity)` that constructs an empty list with the specified initial capacity.
- **The Gap:** We need to implement the native bridge `native_arraylist_init_capacity` (or equivalent) in `native.rs` and register it in `stdlib.rs` for `ArrayList.<init>(I)V`. Since Duke's `ArrayList` is currently backed by a dynamic list of slots (`fields`), the `initialCapacity` argument will likely just initialize the size counter to 0, matching the behavior of the default constructor, as Duke's garbage collector/heap allocation abstracts away internal capacity sizing for fields.

## ✅ Acceptance Criteria
- Must implement the native method for `java/util/ArrayList.<init>(I)V`.
- Must register the native method in `crates/duke-interpreter/src/stdlib.rs` for `java/util/ArrayList`.
- The method must initialize the `ArrayList`'s size to `0` (similar to the default constructor).
- Must handle the integer argument `initialCapacity` correctly. If `initialCapacity < 0`, it should ideally throw a `java.lang.IllegalArgumentException` (to match the Java spec).
- The `slf4j-simple` test blocker `Unsupported native: java/util/ArrayList.<init>(I)V` must be resolved.

## 🚫 Out of Scope
- Modifying the underlying `duke-gc` memory layout to physically pre-allocate exactly `N` array slots, unless strictly necessary for correctness. Duke's fields list is dynamically sized, so logically initializing size to 0 is sufficient for functionality.
- Implementing `ensureCapacity(int)` or `trimToSize()`. Focus only on the constructor.
