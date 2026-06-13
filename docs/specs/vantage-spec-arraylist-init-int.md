# 🔭 Vantage: Spec for `java/util/ArrayList.<init>(I)V`

## 👤 User Story
"As a System initializing real-world Java libraries, I want the JVM to support `ArrayList.<init>(I)V` so that performance-optimized collections with pre-allocated capacities can be created without throwing an 'Unsupported native' exception."

## ❓ The "So What?"
What business problem does this solve?
Standard libraries (like SLF4J and `java.util` itself) heavily use pre-allocated `ArrayList`s to minimize reallocations during collection growth. Without this constructor, any library relying on sizing hints will fail to load, blocking ecosystem compatibility. Specifically, the `slf4j-simple` smoke test is currently hard-blocked on this exact initialization method.

## 📈 Metric Definition
Success = The `slf4j-simple` smoke test successfully progresses past `ArrayList.<init>(I)V` and hits the next capability gap.

## 🔍 Gap Analysis
- **Current State:** Duke currently supports `ArrayList.<init>()V` (default capacity) and `ArrayList.<init>(Collection)V` (copy constructor) via native implementations. However, it lacks the `(I)V` variant (initial capacity).
- **Market/Standard Lib:** Standard JVMs execute this as a bytecode method or a native intrinsic that simply pre-allocates the backing array to avoid dynamic resizing.
- **The Gap:** We need to implement `java/util/ArrayList.<init>(I)V` as a registered native to handle explicit size initializations.

## ✅ Acceptance Criteria
- Must implement `java/util/ArrayList.<init>(I)V`.
- Must correctly handle the initial capacity integer argument. It should allocate the correct Duke-specific layout (e.g., `fields[0]` = size = 0, with sufficient capacity allocated underneath if applicable to Duke's model).
- Must throw an `IllegalArgumentException` if the initial capacity argument is negative (mimicking standard Java behavior).

## 🚫 Out of Scope
- Actually implementing advanced backing storage optimizations for the specific capacity; as long as the collection initializes correctly in Duke's internal layout and does not crash, the sizing hint can act as a no-op or a basic allocation.
