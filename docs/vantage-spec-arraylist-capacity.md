# 🔭 Vantage: Spec for ArrayList Initial Capacity Constructor

## 👤 User Story
"As a Java Application Developer running on Duke, I want to initialize an `ArrayList` with a specific initial capacity, so that my application can pre-allocate memory efficiently and avoid unnecessary dynamic resizing during bulk additions."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke's ArrayList supports the zero-argument constructor and the collection-copy constructor. However, real-world Java libraries frequently optimize their memory usage by instantiating lists with a known initial size, using `new ArrayList<>(int initialCapacity)`. Our real-world JAR compatibility smoke tests (`slf4j-simple`) are explicitly blocked by the lack of `java/util/ArrayList.<init>(I)V`. Without this constructor, Duke cannot load or execute fundamental enterprise libraries, stunting its viability as a real-world runtime. Adding this constructor unlocks the next phase of SLF4J execution and improves compatibility with standard Java idioms.

## 📈 Metric Definition
Success = The `slf4j-simple` smoke test progresses past the `java/util/ArrayList.<init>(I)V` blocker, and a direct test initializing an `ArrayList` with a specific integer capacity (e.g., `new ArrayList<>(10)`) executes correctly, initializing the list's size to 0.

## 🔍 Gap Analysis
- **Current State:** Duke implements standard constructors natively, but attempting to execute `new ArrayList<>(int)` throws an "Unsupported native" or missing method error.
- **Market/Standard Lib:** The standard Java `ArrayList` class provides `ArrayList(int initialCapacity)`. Duke must provide the method signature to satisfy the bytecode linkage and initialize the logical size counter to 0.
- **The Gap:** We need to register and implement the native handler for `java/util/ArrayList.<init>(I)V`.

## ✅ Acceptance Criteria
- Must register `java/util/ArrayList.<init>(I)V` in the standard library native registry.
- Must implement a native handler that takes an `int` argument and correctly initializes the `ArrayList`'s size field to 0.
- The `slf4j-simple` smoke test must progress past the `ArrayList.<init>(I)V` blocker.
- Must execute a test script invoking `new ArrayList<>(42)` without throwing errors, and subsequent `add` and `size` calls must function correctly.

## 🚫 Out of Scope
- Throwing `IllegalArgumentException` on negative capacity. While standard Java does this, we are prioritizing execution flow. (Can be implemented if trivial, but not the primary goal).
- Modifying the underlying dynamic memory model. We retain the synthetic representation.
