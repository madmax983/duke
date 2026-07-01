# 🔭 Vantage: Spec for ArrayList Capacity

## 👤 User Story
"As a Java Developer running third-party libraries (like slf4j) on Duke, I want the VM to support initializing an `ArrayList` with a specific initial capacity (`ArrayList.<init>(I)V`), so that my libraries can optimize memory usage and avoid unnecessary reallocations when they know the expected size of a list."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke's interpreter provides native implementations for `ArrayList.<init>()V` (default constructor) and `ArrayList.<init>(Collection)V`. However, it lacks support for `ArrayList.<init>(I)V`, which takes an initial capacity integer. Many common Java libraries, including `slf4j-simple`, explicitly call this constructor to pre-allocate memory when parsing configurations or managing internal structures. Without this native implementation, any class (like `slf4j`'s internal data structures) that uses this specific constructor will fail during execution with a missing native exception, halting the entire application. Implementing this unblocks the `slf4j-simple` OSS JAR smoke test and enables a broader class of standard libraries to run.

## 📈 Metric Definition
Success = A user can instantiate `new ArrayList(10)`, load the class in Duke, and execute it without encountering a missing native error. The `slf4j-simple` OSS JAR smoke test must progress past the `java/util/ArrayList.<init>(I)V` blocker.

## 🔍 Gap Analysis
- **Current State:** The `duke-interpreter` crate registers `ArrayList.<init>()V` and `ArrayList.<init>(Collection)V` in `stdlib.rs`. When a program invokes `ArrayList.<init>(I)V`, the interpreter attempts to resolve it and fails because it is unregistered.
- **Market/Standard Lib:** Standard JVMs (and the Java SE API) provide this constructor to allow setting the initial array capacity. Although Duke's internal representation of `ArrayList` uses dynamically growable fields (without an explicit underlying fixed-size array in the same way the OpenJDK does), it must still accept the capacity argument to maintain API compatibility.
- **The Gap:** We need to update `duke-interpreter`'s `stdlib.rs` to register the `(I)V` constructor for `ArrayList` and implement a corresponding native function (e.g., `native_arraylist_init_capacity`) in `native.rs` that accepts the capacity integer and initializes the list. Given Duke's dynamic field implementation, this capacity can either be ignored or used to pre-allocate internal field slots if supported.

## ✅ Acceptance Criteria
- `duke-interpreter` must register the `java/util/ArrayList.<init>(I)V` native in `stdlib.rs`.
- Must implement the native function in `native.rs` that correctly unpacks the `ArrayList` reference and the integer capacity argument.
- The implementation must properly initialize the `ArrayList` state (e.g., setting the `size` field to 0).
- If the capacity is negative, it must throw an `IllegalArgumentException` as required by the Java specification.
- The `slf4j-simple` smoke test must progress to its next blocker or pass.

## 🚫 Out of Scope
- Re-architecting Duke's underlying `ArrayList` implementation from dynamic fields to a strict fixed-size array just to match OpenJDK internals.
- Implementing other missing `ArrayList` methods not strictly related to instantiation and basic capacity handling.