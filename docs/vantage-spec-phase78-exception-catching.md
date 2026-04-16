# 🔭 Vantage: Spec for Exception Catching (NPE, AIOOB, CCE, StackOverflow)

## 👤 User Story
"As a Java Developer running my code on Duke, I want the VM to properly catch and handle standard runtime exceptions like `NullPointerException`, `ArrayIndexOutOfBoundsException`, `ClassCastException`, and `StackOverflowError`, so that my applications can recover gracefully from errors instead of crashing the entire VM."

## ❓ The "So What?"
What business problem does this solve?
Robust applications expect the runtime to enforce boundaries and allow programmatic recovery via `try/catch` blocks. Currently, boundary violations or null dereferences might crash the VM or halt execution entirely. Without reliable exception generation and catching for these standard fault conditions, developers cannot write resilient software. Supporting implicit exception throwing and catching guarantees that Duke behaves like a standard, safe JVM, preserving uptime and allowing business logic to handle its own faults.

## 📈 Metric Definition
Success = A Java program running on Duke that triggers an implicit fault (e.g., dereferencing a null reference, accessing an array out of bounds, casting to an incompatible type, or overflowing the call stack) will correctly construct the corresponding Exception object and branch to the appropriate `catch` block, ultimately returning the expected value (e.g., successfully passing the probes in `Phase78Test`).

## 🔍 Gap Analysis
- **Current State:** Duke has basic exception throwing and catching mechanisms, but lacks comprehensive, implicit generation and routing for standard runtime exceptions (NPE, AIOOB, CCE, StackOverflow) triggered by bytecode instructions.
- **Market/Standard Lib:** The standard JVM strictly specifies that certain instructions (like `aload`, `iaload`, `checkcast`, method invocations) must implicitly throw specific exceptions when invariants are violated.
- **The Gap:** We need to update the execution loop and memory access patterns to detect these faults safely and translate them into Java-level exception objects that integrate with the existing exception routing mechanism.

## ✅ Acceptance Criteria
- Must throw and catch `java.lang.NullPointerException` when a null reference is dereferenced (e.g., getting a field, calling a method, or accessing an array).
- Must throw and catch `java.lang.ArrayIndexOutOfBoundsException` when an array is accessed with an invalid index.
- Must throw and catch `java.lang.ClassCastException` during a `checkcast` instruction if the object is not compatible with the target type.
- Must throw and catch `java.lang.StackOverflowError` when the method call stack exceeds the configured maximum depth.
- The `Phase78Test` integration tests (which cover NPE, AIOOB, CCE, and StackOverflow) must pass successfully.

## 🚫 Out of Scope
- Detailed stack trace generation with file names and line numbers for these implicit exceptions (basic exception generation and routing is the priority).
- Asynchronous exceptions (e.g., `Thread.stop()`).
