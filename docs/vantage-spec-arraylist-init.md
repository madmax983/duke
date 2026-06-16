# 🔭 Vantage: Spec for ArrayList Initial Capacity Constructor

## 👤 User Story
"As a Developer running standard Java libraries on Duke, I want the JVM to support `ArrayList.<init>(I)V` so that collections can be instantiated with an upfront sizing hint without throwing an 'Unsupported native' exception."

## ❓ The "So What?"
What business problem does this solve?
Performance-sensitive libraries (including SLF4J and the JDK itself) heavily use pre-allocated `ArrayList`s (e.g., `new ArrayList<>(10)`) to minimize array copying during collection growth. Without this constructor, any library relying on sizing hints will fail to load, blocking ecosystem compatibility. Specifically, the `slf4j-simple` real-world smoke test is currently hard-blocked on this exact initialization method. Implementing this allows Duke to execute standard, performance-oriented third-party code.

## 📈 Metric Definition
Success = The `slf4j-simple` OSS smoke test progresses past the `ArrayList.<init>(I)V` failure and discovers the *next* capability gap.

## 🔍 Gap Analysis
- **Current State:** Duke currently supports `ArrayList.<init>()V` (default capacity) and `ArrayList.<init>(Collection)V` (copy constructor). However, it lacks the explicit `(I)V` variant (initial capacity).
- **Market/Standard Lib:** Standard JVMs execute this constructor to eagerly allocate the backing array, avoiding dynamic resizing overhead when the expected element count is known.
- **The Gap:** We need to implement `java/util/ArrayList.<init>(I)V` as a registered native. Because Duke's `ArrayList` memory layout differs from standard Java (Duke uses dynamic `Vec` fields on the heap object rather than a separate `Object[]` array), the sizing hint will primarily act as a compatibility stub or, at best, a pre-allocation hint (`Vec::with_capacity`) for the internal fields array.

## ✅ Acceptance Criteria
- Must implement and register the native `java/util/ArrayList.<init>(I)V`.
- Must parse the initial capacity integer argument from the execution frame.
- Must initialize the `ArrayList` object, setting its internal size tracker (`fields[0]`) to `0`.
- Must throw a `java/lang/IllegalArgumentException` if the provided initial capacity is negative, matching the standard Java contract.
- Must execute without crashing when a valid (non-negative) capacity is provided.

## 🚫 Out of Scope
- Actually enforcing a maximum capacity limit.
- Implementing complex array copying or resizing logic, since Duke's `Heap` handles variable-length field arrays dynamically. The size hint only needs to satisfy the API signature and prevent negative capacity errors.
