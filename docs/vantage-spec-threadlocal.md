# 🔭 Vantage: Spec for ThreadLocal Support

## 👤 User Story
"As a Backend Engineer running modern Java frameworks on Duke, I want `java.lang.ThreadLocal` to function correctly, so that I can maintain thread-safe state without passing context objects manually throughout my architecture."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke JVM provides a synthetic stub for `java.lang.ThreadLocal` and its subclass `InheritableThreadLocal`. However, this is insufficient for executing real-world Java applications and frameworks (e.g., Spring Boot, Gson, SLF4J), which heavily rely on true thread-local storage for transaction scopes, web request contexts, and logging contexts. The lack of genuine thread-level isolation prevents concurrent enterprise applications from functioning correctly on Duke. Implementing real `ThreadLocal` storage natively unlocks standard Java enterprise compatibility and enables safe parallel execution.

## 📈 Metric Definition
Success = A Java program can execute parallel threads that set, get, and remove distinct values using the same `ThreadLocal` instance, and each thread reads its own isolated value correctly without data corruption or panicking.

## 🔍 Gap Analysis
- **Current State:** Duke maps `ThreadLocal` to a synthetic single-slot holder where `field 0 = value`, meaning all threads share the same slot. `InheritableThreadLocal` is a no-op that behaves identically.
- **Market/Standard Lib:** The standard JVM's `ThreadLocal` maintains a map of values localized to the current executing `Thread` instance.
- **The Gap:** We need a native representation of thread-local state within Duke's threading model and memory heap, along with the native method implementations for `ThreadLocal.get()`, `set()`, and `remove()` that lookup values based on the currently executing thread.

## ✅ Acceptance Criteria
- Must natively implement `ThreadLocal.set(Object)` isolating the value to the current thread.
- Must natively implement `ThreadLocal.get()` returning the value for the current thread or `null` if uninitialized.
- Must natively implement `ThreadLocal.remove()` clearing the thread's value.
- Must natively implement `InheritableThreadLocal` ensuring child threads inherit the parent's values upon creation.
- Execution of Gson or Spring Boot contexts relying on `ThreadLocal` must succeed concurrently.

## 🚫 Out of Scope
- Optimizing `ThreadLocalMap` internals to exactly mirror HotSpot's hash-based map implementation; a functionally correct map linked to the thread scope is sufficient for Phase 1.
