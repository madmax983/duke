# 🔭 Vantage: Spec for ArrayList Initial Capacity Support

## 👤 User Story
"As a Java Library Developer or User running my code on Duke, I want the VM to support `java.util.ArrayList.<init>(I)V`, so that I can initialize lists with a specific capacity to optimize memory allocation and performance when the size is known in advance."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke JVM is blocked from executing real-world JARs, specifically `slf4j-simple`, because it does not implement the native method `java.util.ArrayList.<init>(I)V` (the ArrayList constructor that takes an initial capacity). The standard library heavily relies on initializing collections with pre-computed sizes to prevent dynamic heap reallocation overhead as the collection grows. Without supporting this constructor, Duke crashes on standard initialization sequences, limiting its compatibility with the broader Java ecosystem. Supporting this constructor unblocks the SLF4J smoke test and represents a crucial step in proving compatibility with real-world enterprise libraries.

## 📈 Metric Definition
Success = The Duke JVM can successfully execute `new java.util.ArrayList(10)`, and the integration test `slf4j_simple_smoke_surfaces_next_missing_capability_explicitly` advances past the current `Unsupported native: java/util/ArrayList.<init>(I)V` error.

## 🔍 Gap Analysis
- **Current State:** Duke has basic support for `ArrayList` via synthetic classes and native methods like `<init>()V` (default constructor) and `<init>(Collection)V`. However, it lacks support for the `<init>(I)V` constructor.
- **Market/Standard Lib:** The standard JVM `ArrayList` uses `int` capacities to pre-allocate backing arrays.
- **The Gap:** We need to implement the native method bridge for `java.util.ArrayList.<init>(I)V` in Duke's interpreter so that Java code attempting to pre-allocate capacity does not trigger a method not found panic.

## ✅ Acceptance Criteria
- Must implement the native bridge `java.util.ArrayList.<init>(I)V`.
- Must properly initialize the `ArrayList` object, maintaining compatibility with the existing internal layout (e.g., setting the size field to 0).
- Must optionally handle pre-allocation logic if the underlying Duke `HeapObject.fields` supports pre-reserving capacity (e.g., `Vec::with_capacity()`), or otherwise handle it gracefully.
- The `slf4j-simple` smoke test must progress past the `java/util/ArrayList.<init>(I)V` blocker.

## 🚫 Out of Scope
- Complete implementation of dynamic array resizing logic in `add()` if it's not strictly necessary to fulfill the capacity constructor contract.
- Changing the internal structural representation of `ArrayList` within Duke.
