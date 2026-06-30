# 🔭 Vantage: Spec for ArrayList Support

## 👤 User Story
As a Developer running third-party Java libraries (like `slf4j`), I want the JVM to support `java.util.ArrayList` initialization and core operations, so that I can use standard Java collections and my third-party dependencies can successfully start up without crashing the VM.

## 💼 The "So What?" (Business Problem)
Without standard library collection support, our JVM is limited to executing toy programs and simple algorithmic code. Real-world enterprise Java libraries rely heavily on `java.util.ArrayList` for dynamic array management. The current `slf4j-simple` OSS smoke test is explicitly blocked at `java/util/ArrayList.<init>(I)V`. Unblocking this is the critical path to running real-world OSS jars, which expands the utility and market fit of our JVM. Features are liabilities until they are used, and right now, our lack of collection support makes the JVM unusable for most Java developers.

## 📊 Metric Definition
- **Success:** The `slf4j-simple` OSS smoke test successfully progresses past `java/util/ArrayList.<init>(I)V`.
- **Quality:** Instantiation of `ArrayList` and its backing array is performed correctly according to JVM memory safety rules.
- **Stability:** Passing the relevant standard library unit tests with 0 panics.

## 🔍 Gap Analysis
- **Current State:** The VM halts execution when hitting missing native methods or synthetic classes related to `java.util.ArrayList` (specifically the constructor).
- **Market Standard:** HotSpot and OpenJ9 support highly optimized, fully featured dynamic array structures via standard library bytecode.
- **Our Approach:** We need to provide the foundational support to handle `java/util/ArrayList.<init>(I)V` to unblock real-world library initialization.

## ✅ Acceptance Criteria
- Must handle the `java/util/ArrayList.<init>(I)V` (constructor with initial capacity integer argument) without panicking.
- Must correctly support the allocation of the backing storage (e.g., `Object[]`) based on the requested capacity.
- Must throw `IllegalArgumentException` (or halt gracefully) if the specified initial capacity is negative, matching the Java SE 21 specification.

## 🚫 Out of Scope
- Full `java.util.Collections` framework support (Phase 2).
- Thread-safe variants like `Vector` or `CopyOnWriteArrayList` (Not needed for this specific blocker).
- Stream API integration for ArrayList.
