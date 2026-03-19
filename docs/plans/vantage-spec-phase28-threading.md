# 🔭 Vantage: Spec for Basic Threading (`java.lang.Thread`, `Runnable`)

## 👤 User Story
"As a Java Developer running on Duke, I want to execute tasks concurrently using `java.lang.Thread` and `Runnable`, so that my applications can perform background processing, handle multiple clients, and improve throughput."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke executes Java bytecode strictly sequentially in a single thread. Modern Java applications inherently rely on concurrency—even simple web servers or background job processors require multiple threads to function efficiently and avoid blocking the main execution path. Without support for basic threading, Duke cannot run most enterprise or server-side Java workloads. Adding support for `java.lang.Thread` allows Duke to execute real-world, concurrent applications, vastly expanding its addressable market and utility.

## 📈 Metric Definition
Success = A Java program running on Duke can successfully spawn at least 10 concurrent threads that implement `Runnable`, each printing a message and sleeping via `Thread.sleep()`, and the main thread can successfully wait for them to finish using `Thread.join()`.

## 🔍 Gap Analysis
- **Current State:** Duke has a single execution thread. `java.lang.Thread.start0()` native method is unimplemented, meaning any attempt to start a thread will fail or do nothing.
- **Market/Standard Lib:** Standard Java concurrency is built on top of `java.lang.Thread`. The JVM must map these Java-level threads to host OS threads (1:1 mapping is standard).
- **The Gap:** We need native implementations for thread creation, lifecycle management (start, join, sleep), and a mapping between the Java `Thread` object and a host Rust thread running the interpreter loop.

## ✅ Acceptance Criteria
- Must implement `java.lang.Thread.start0()` to spawn a new native OS thread (via Rust's `std::thread`).
- The new thread must correctly invoke the `run()` method of the target `Runnable` or `Thread`.
- Must implement `java.lang.Thread.sleep()` to pause the current thread without blocking other threads.
- Must implement `java.lang.Thread.join()` to allow one thread to wait for another to complete.
- Must support basic thread state management (New, Runnable, Terminated).
- Must ensure that the JVM does not exit until all non-daemon threads have finished executing.

## 🚫 Out of Scope
- Advanced synchronization primitives (e.g., `Object.wait()`, `Object.notify()`, `synchronized` blocks/methods) - these belong in a separate locking phase.
- `java.util.concurrent` advanced utilities (e.g., Thread Pools, Executors).
- Thread interruption (`Thread.interrupt()`).
- Daemon threads (`Thread.setDaemon()`).

## Implementation Status (2026-03-19)
- Implemented Phase 28 thread lifecycle support with synthetic `java/lang/Thread` and `java/lang/Runnable` bootstrap classes plus native `start`, `join`, and `sleep` handling.
- Duke now routes threaded entrypoints through `execute_class_to_completion()`, which keeps the VM alive until spawned worker threads finish.
- Execution uses host OS threads, but bytecode evaluation still runs under one shared interpreter critical section, so this phase provides lifecycle correctness rather than parallel bytecode throughput.
- `monitorenter` and `monitorexit` remain no-ops, and allocation-triggered GC is suppressed while worker threads are live.
- Still out of scope: daemon threads, interruption, `wait`/`notify`, and full monitor semantics.
