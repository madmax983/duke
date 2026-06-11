# 🔭 Vantage: Spec for ArrayList Initial Capacity

## 👤 User Story
"As a Backend Engineer using Duke, I want to initialize `ArrayList` with a specific capacity (`ArrayList.<init>(I)V`), so that I can prevent unnecessary dynamic resizing and reduce GC pressure when the expected size is known."

## ❓ The "So What?"
What business problem does this solve?
Real-world Java applications, including critical libraries like SLF4J, frequently optimize list creations by specifying an initial capacity. Lacking this constructor blocks these standard libraries from running on Duke, completely halting execution with `Unsupported native: java/util/ArrayList.<init>(I)V`. Unblocking this method is on the critical path for broad OSS compatibility.

## 📈 Metric Definition
Success = A user can instantiate `new java.util.ArrayList(10)` without throwing an `UnsatisfiedLinkError` or interpreter panic, successfully progressing past the current execution blocker in the `slf4j-simple` test suite.

## 🔍 Gap Analysis
- **Current State:** Duke supports `ArrayList.<init>()` and `ArrayList.<init>(Collection)`, but lacks the `(I)V` variant. Code calling `new ArrayList(10)` crashes.
- **Market/Standard Lib:** Standard JVMs fully support capacity-aware initializations to pre-allocate backing arrays.
- **The Gap:** We need to implement the `ArrayList.<init>(I)V` native method in the registry.

## ✅ Acceptance Criteria
- Must implement the native method `java/util/ArrayList.<init>(I)V`.
- The implementation must initialize the `ArrayList` correctly. Due to Duke's slot-based field layout for `ArrayList`, it can functionally behave identically to the no-arg constructor (e.g. setting size to 0).
- Must allow execution to proceed past the `ArrayList` blocker in integration testing.

## 🚫 Out of Scope
- Real underlying array pre-allocation (since Duke dynamically manages array fields differently).
- Memory footprint optimization of the backing slots for this specific phase.
