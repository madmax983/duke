# 🔭 Vantage: Spec for Virtual Threads (Project Loom)

## 👤 User Story
"As a Backend Developer writing high-concurrency microservices, I want to use Java 21 Virtual Threads (`java.lang.Thread.ofVirtual()`), so that I can handle millions of concurrent blocking I/O operations without the memory overhead and OS-level context switching costs of traditional platform threads."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke maps Java threads 1:1 with OS threads (platform threads). In high-throughput applications, threads block frequently on network or database calls. OS threads are heavy (often requiring a 1MB stack) and context switching is expensive, capping the number of concurrent connections a service can handle before running out of memory or thrashing the CPU. Virtual Threads (introduced in Java 21) solve this by decoupling the Java thread from the OS thread, allowing millions of lightweight virtual threads to multiplex onto a small pool of carrier OS threads. Supporting Virtual Threads is essential for Duke to be competitive as a runtime for modern, massively concurrent backend services (like those built with Spring Boot 3+).

## 📈 Metric Definition
Success = A user can create and execute 1,000,000 virtual threads concurrently in a simple Java program running on Duke without triggering an `OutOfMemoryError` or taking significantly longer than a standard JVM (e.g., execution completes within a reasonable timeframe, bounded by CPU rather than OS thread limits).

## 🔍 Gap Analysis
- **Current State:** Duke implements standard platform threads using OS threads. Any code attempting to use the `VirtualThread` API or related internal Continuations will fail because the underlying VM mechanics for stack swapping and mounting/unmounting are missing.
- **Market/Standard Lib:** Java 21 standardizes Virtual Threads. The implementation relies on internal JVM support for `jdk.internal.vm.Continuation` to freeze and thaw execution stacks, and `java.util.concurrent.ForkJoinPool` for the carrier thread scheduler.
- **The Gap:** Duke needs to implement the low-level VM support for capturing, storing, and restoring execution frames (Continuations) so that a virtual thread can yield execution when it encounters a blocking operation and resume later on a different carrier thread.

## ✅ Acceptance Criteria
- Must support the creation and execution of virtual threads via `Thread.ofVirtual().start(...)` and `Executors.newVirtualThreadPerTaskExecutor()`.
- Must implement the native primitives required for `jdk.internal.vm.Continuation` (yielding, freezing, and thawing execution stacks).
- Must ensure that blocking I/O operations (e.g., `Socket.read()`, `Thread.sleep()`) correctly yield the underlying carrier thread instead of blocking it.
- Must ensure `ThreadLocal` variables work correctly and independently within the context of a virtual thread.
- Must pass standard Java 21 TCK (Technology Compatibility Kit) tests related to Virtual Threads.

## 🚫 Out of Scope
- Custom scheduling algorithms for carrier threads (we will rely on the standard library's `ForkJoinPool`).
- Retrofitting legacy blocking APIs (like old `java.io` components) if they are not updated by the standard library to support virtual threads (we rely on the standard library's implementations).
