# 🔭 Vantage: Spec for java.util.ArrayList.<init>(I)V

## 👤 User Story
"As a Java Developer running my code on Duke, I want to initialize an `ArrayList` with a specific initial capacity, so that I can avoid unnecessary array resizing overhead when I know the number of elements in advance."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke fails to execute third-party Java libraries (like `slf4j-simple`) that rely on pre-sizing `ArrayList` instances. The constructor `java.util.ArrayList.<init>(I)V` is a fundamental part of the Java Collections Framework. Implementing this constructor is a direct unblocker for real-world JAR compatibility. Without it, any application using this common constructor will crash, rendering Duke unusable for a wide range of standard applications.

## 📈 Metric Definition
Success = Duke successfully executes `java/util/ArrayList.<init>(I)V` without throwing a `NoSuchMethodError` or panicking. The `slf4j-simple` compatibility test must progress past this current blocker.

## 🔍 Gap Analysis
- **Current State:** Duke's interpreter or standard library stubs do not handle the `ArrayList(int initialCapacity)` constructor, halting execution of libraries like `slf4j`.
- **Market/Standard Lib:** In the OpenJDK standard library, this constructor initializes the internal `elementData` array to the requested size, throwing an `IllegalArgumentException` if the size is negative.
- **The Gap:** We need to provide the implementation (whether through native bridging, bytecode injection, or standard library inclusion) that properly initializes the `ArrayList`'s internal state when given an integer capacity.

## ✅ Acceptance Criteria
- Must correctly execute the `java.util.ArrayList.<init>(I)V` constructor.
- Must handle the integer argument `initialCapacity`.
- Must allocate the underlying array to the specified `initialCapacity`.
- Must throw `java.lang.IllegalArgumentException` if the provided `initialCapacity` is less than 0.
- Must correctly handle an `initialCapacity` of 0.

## 🚫 Out of Scope
- Full implementation of all `ArrayList` methods. This spec is strictly scoped to the `(I)V` constructor.
- Performance optimization of the internal array growth strategy (e.g., `grow()` method).
