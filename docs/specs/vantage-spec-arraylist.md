# 🔭 Vantage: Spec for java.util.ArrayList

## 👤 User Story
"As a Java Developer running third-party code (like `slf4j-simple`), I want the JVM to support `java.util.ArrayList`, so that common operations like dynamic list creation, iteration, and element access can execute without blocking application startup or crashing the VM."

## 🤔 So What?
**What business problem does this solve?**
`ArrayList` is the most ubiquitous collection type in the Java ecosystem. Without it, almost no real-world Java applications or libraries (such as logging frameworks, JSON parsers, and web servers) can initialize. Implementing `ArrayList` removes a major blocker preventing OSS JAR compatibility (specifically Issue #854 where `slf4j-simple` is blocked at `ArrayList.<init>(I)V`). This directly unlocks our ability to run standard Java ecosystem tools.

## 📈 Metric Definition
**Success =**
- The JVM successfully initializes `java.util.ArrayList` without throwing an `UnsupportedOperationException` or `NoClassDefFoundError`.
- Third-party OSS smoke tests (e.g., `slf4j-simple`) can progress past the current blocker (`java/util/ArrayList.<init>(I)V`).
- Basic `add`, `get`, `size`, and `iterator` operations execute with O(1) amortized time complexity.

## 🕳️ Gap Analysis
Currently, Duke JVM lacks a functional implementation for `java.util.ArrayList` initialization and its core operations. When standard libraries attempt to allocate an `ArrayList` with an initial capacity, execution halts. The standard library provides these implementations, but our runtime needs the native/internal backing and proper class loading support to execute the bytecode.

## ✅ Acceptance Criteria
- Must support constructor `ArrayList(int initialCapacity)`.
- Must support constructor `ArrayList()`.
- Must support constructor `ArrayList(Collection<? extends E> c)`.
- Core methods (`add`, `get`, `set`, `remove`, `size`, `isEmpty`, `clear`) must function correctly.
- Must dynamically resize its internal array when capacity is reached.
- Must return a valid Iterator/ListIterator that supports concurrent modification checks (fail-fast).
- Must handle null elements properly.

## 🚫 Out of Scope
- Implementation of specialized primitive collections (e.g., `IntArrayList`).
- Highly optimized vector API (SIMD) implementations for bulk operations (Phase 2).
- Complete implementations of `Spliterator` and `Stream` support for `ArrayList` (handled in separate Stream API specifications).
