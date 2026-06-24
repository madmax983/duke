# 🔭 Vantage: Spec for `java.util.ArrayList.<init>(I)V`

## 👤 User Story
"As a Java Application Developer running my code on Duke, I want the VM to support the `java.util.ArrayList` constructor taking an initial capacity (`<init>(I)V`), so that my applications can pre-allocate memory and optimize list performance, and third-party libraries like SLF4J can successfully run."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke's JVM is blocked from executing the `slf4j-simple` library because it encounters the `java.util.ArrayList.<init>(I)V` constructor. Modern Java frameworks and applications frequently initialize `ArrayList` with a specific capacity to avoid unnecessary array resizing overhead. Without support for this specific constructor, any JAR that instantiates `ArrayList` with an initial capacity will fail to execute. Supporting it unblocks critical logging libraries and a massive portion of the broader Java ecosystem.

## 📈 Metric Definition
Success = A user can execute Java bytecode containing `new java.util.ArrayList(int)` on Duke without encountering a `NoSuchMethodError` or panic, and the SLF4J smoke test progresses past its current block.

## 🔍 Gap Analysis
- **Current State:** Duke has progressed past `String.equalsIgnoreCase` but is explicitly blocked on `java.util.ArrayList.<init>(I)V` according to the oss-jars smoke test for SLF4J.
- **Market/Standard Lib:** The standard JVM provides `java.util.ArrayList` with overloaded constructors, including the one for initial capacity which initializes the backing array.
- **The Gap:** We need to provide the synthetic class definition and/or native bridge logic for the `ArrayList(int initialCapacity)` constructor so that Duke can properly resolve and execute the instruction.

## ✅ Acceptance Criteria
- Must provide the synthetic representation or native implementation for `java.util.ArrayList.<init>(I)V`.
- Must successfully execute bytecode that instantiates an `ArrayList` with a positive integer.
- Must throw a `java.lang.IllegalArgumentException` if the provided capacity is negative, mimicking the standard JVM behavior.
- The SLF4J smoke test (`oss_jar_smoke`) must progress past the `java.util.ArrayList.<init>(I)V` blocker.

## 🚫 Out of Scope
- Complete implementation of the entire `java.util.ArrayList` API (e.g., `add`, `get`, `remove`, `iterator`). We only need what is strictly required to unblock the constructor and the immediate execution path.
- Optimizing the backing array allocation within the Duke JVM memory model for this phase.
