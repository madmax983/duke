# 🔭 Vantage: Spec for Java Collections Framework Support

## 👤 User Story
As a Developer, I want to use standard Java collections (like `ArrayList`, `HashMap`), so that I don't have to reinvent basic data structures and can run real-world Java libraries.

## ❓ The "So What?"
**What business problem does this solve?**
The JVM currently fails on the `slf4j-simple` OSS smoke test because it lacks support for `java.util.ArrayList.<init>(I)V`. Without robust support for `java.util.*` collections, almost no third-party Java applications or libraries can run. Implementing the underlying support for these unlocks massive ecosystem compatibility, moving Duke closer to being a usable runtime.

## 📊 Metric Definition
**Success =**
- OSS smoke test for `slf4j-simple` progresses past the `java/util/ArrayList.<init>(I)V` blocker.
- Can instantiate, add items to, and iterate over an `ArrayList` and `HashMap` in basic test fixtures without crashing.
- Core array operations (`anewarray`, `aastore`, `aaload`) execute correctly.

## 🔍 Gap Analysis
Currently, Duke fails when encountering basic collection initialization. Standard libraries like OpenJDK provide implementations of these collections that rely on native object arrays and specific JVM mechanisms (like array resizing and `System.arraycopy`). We need to ensure the JVM provides the necessary underlying native methods and runtime array support that `java.util` collections expect to function.

## ✅ Acceptance Criteria
- Must successfully execute `java/util/ArrayList.<init>(I)V`.
- Must implement or correctly bridge `System.arraycopy` to support fast array resizing.
- Must support array allocation and manipulation opcodes (`newarray`, `anewarray`, `aastore`, `aaload`, `arraylength`) sufficient to back `ArrayList`.
- Must support basic `Object.hashCode()` dispatching required by `HashMap`.

## 🚫 Out of Scope
- Concurrent collections (`java.util.concurrent.*`) such as `ConcurrentHashMap`.
- Full stream API (`java.util.stream.*`) integration.
- Specialized collections like `WeakHashMap`, `EnumMap`, or `TreeMap`.
