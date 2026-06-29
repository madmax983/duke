# 🔭 Vantage: Spec for ArrayList.<init>(I)V

## 👤 User Story
"As a Java Developer running my code on Duke, I want to initialize an `ArrayList` with an initial capacity using `new ArrayList(int initialCapacity)`, so that I can optimize memory allocations and avoid resizing overhead when the expected list size is known."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke's synthetic `ArrayList` implementation lacks support for the `ArrayList.<init>(I)V` constructor. This constructor is extremely common in performance-conscious Java applications where lists are pre-allocated to avoid multiple array resizing operations. Without it, standard third-party libraries (like `slf4j`, as discovered in the `slf4j-simple` OSS smoke tests) will fail to initialize. Implementing this constructor unblocks the execution of widely used libraries and improves the VM's compatibility with standard Java bytecode. Complexity here is minimal, as it primarily involves registering the existing `native_arraylist_init` behavior (which ignores capacity in the dynamic slot layout) for the `(I)V` signature.

## 📈 Metric Definition
Success = The `slf4j-simple` OSS smoke test progresses past the `java/util/ArrayList.<init>(I)V` blocker, and a dedicated test verifying `new ArrayList<>(10)` successfully instantiates a list without throwing an exception.

## 🔍 Gap Analysis
- **Current State:** Duke's synthetic `ArrayList` supports the no-arg constructor `()` and the collection constructor `(Ljava/util/Collection;)V`. It throws a `NoSuchMethodError` when `new ArrayList(int)` is called.
- **Market/Standard Lib:** The standard Java `ArrayList` provides an `(int initialCapacity)` constructor to pre-allocate its internal backing array.
- **The Gap:** Duke uses a dynamic slot-based layout (`fields[0]` = size, `fields[1..]` = elements) rather than a fixed-size backing array. Therefore, the concept of "capacity" is not strictly required for correctness in Duke's architecture. However, the *signature* must exist and execute successfully to allow Java code to run.

## ✅ Acceptance Criteria
- Must register the `java/util/ArrayList.<init>(I)V` native method in the standard library.
- Must correctly parse the integer argument (initial capacity) but can safely discard it because Duke's dynamic heap fields dynamically grow as elements are added.
- Must set the `ArrayList`'s size (at `fields[0]`) to `0` upon initialization, identical to the no-arg constructor.
- Must not throw an exception when a valid non-negative integer is passed.
- Must throw `IllegalArgumentException` if the provided initial capacity is negative, to adhere to the Java specification.

## 🚫 Out of Scope
- Actually pre-allocating unused field slots in Duke's heap. Since Duke handles fields dynamically and dynamically grows them upon `add`, physical memory pre-allocation for slots is unnecessary and out of scope for this compatibility fix.
