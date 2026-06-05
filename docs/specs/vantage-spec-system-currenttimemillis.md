# 🔭 Vantage: Spec for java.lang.System.currentTimeMillis()

## 👤 User Story
"As a backend developer using Duke JVM, I want `System.currentTimeMillis()` to return the current system time in milliseconds, so that my applications can accurately measure elapsed time, schedule tasks, and log timestamps."

## ❓ The "So What?"
What business problem does this solve?
Currently, `System.currentTimeMillis()` is unimplemented or mocked in the interpreter (indicated by a TODO in the codebase). Any standard Java library, framework (like Spring or SLF4J), or application attempting to perform essential tasks such as logging, timeout enforcement, thread sleeping, or performance profiling will fail or behave unpredictably. Without an accurate system clock bridge, Duke JVM cannot run realistic real-world applications. Implementing this native method unlocks compatibility with the vast majority of Java workloads that require time awareness.

## 📈 Metric Definition
Success = `System.currentTimeMillis()` executes successfully without panicking or returning mocked values.
Success = The returned `long` value is strictly increasing across consecutive calls, matching the host OS's epoch time within acceptable timing precision limits (e.g., +/- 50ms).

## 🔍 Gap Analysis
- **Current State:** The JVM interpreter lacks a robust native bridge for `System.currentTimeMillis()`, currently flagged as a `TODO`.
- **Market/Standard Lib:** Every production-grade JVM (HotSpot, GraalVM) natively hooks into the host OS's high-resolution clock to provide epoch milliseconds.
- **The Gap:** We need to implement the JVM native bridge `java/lang/System.currentTimeMillis()J` to fetch the system's current UNIX epoch time in milliseconds and return it to the JVM.

## ✅ Acceptance Criteria
- Must implement `java/lang/System.currentTimeMillis()` returning a valid `long` (Java primitive).
- Must return the number of milliseconds elapsed since the UNIX epoch (January 1, 1970 00:00:00 UTC).
- Must utilize a cross-platform approach to fetch the system time, ensuring compatibility across all supported operating systems.
- Consecutive calls must never go backwards (assuming no external OS clock adjustments).
- Must execute efficiently without significant allocation overhead.

## 🚫 Out of Scope
- High-resolution monotonic timers (`System.nanoTime()`) are deferred to a separate spec.
- Date/Time API extensions (`java.time.*`).
