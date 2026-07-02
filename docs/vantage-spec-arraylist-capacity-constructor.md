# 🔭 Vantage: Spec for ArrayList Capacity Constructor

## 👤 User Story
As a Java Application Developer running on Duke JVM, I want to instantiate `ArrayList` with an initial capacity via `ArrayList.<init>(I)V`, so that my applications and dependencies (like `slf4j-simple`) can optimize their memory allocations and execute without hitting "Unsupported native" barriers.

## 🤔 So What? (Business Problem)
Currently, Duke JVM lacks support for the explicit capacity constructor of `java.util.ArrayList`. This blocks the execution of widely used libraries such as `slf4j-simple`, halting adoption of our JVM for standard real-world JARs. Supporting this constructor unlocks a broader range of ecosystem compatibility.

## 🎯 Metric Definition
- **Success:** The `oss_jar_smoke` test for `slf4j-simple` progresses past `java/util/ArrayList.<init>(I)V`.

## ✅ Acceptance Criteria
- Must register the native method `java/util/ArrayList.<init>(I)V` in the interpreter.
- Must correctly handle the initial capacity integer argument (e.g., rejecting negative capacities).
- Must initialize the underlying `ArrayList` object layout appropriately (size field = 0).

## 🚫 Out of Scope
- Actual pre-allocation of the underlying array buffer, as Duke JVM's current `ArrayList` implementation relies on `HeapObject` dynamic sizing rather than explicit internal `Object[]` buffers.
- Implementation of other `ArrayList` constructors such as `<init>(Collection)V`.
