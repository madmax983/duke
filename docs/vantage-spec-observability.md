# 🔭 Vantage: Spec for Observability & Management (MXBeans)

## 👤 User Story
"As a Platform Engineer operating Duke in production, I want built-in observability primitives (like MemoryMXBean and ThreadMXBean) so that I can monitor heap usage, track garbage collection, and diagnose thread deadlocks without stopping the VM."

## ❓ The "So What?"
What business problem does this solve?
Operating a black-box runtime is a massive liability in production. When a system slows down or crashes, engineers need to know if the JVM ran out of memory, spent all its time in GC, or suffered a thread deadlock. Without built-in observability APIs (`java.lang.management`), APM tools (like Datadog, New Relic) and standard JVM utilities (like `jstat`, `jcmd`) cannot function. By providing Management Beans (MXBeans), Duke proves it is a mature, operations-friendly runtime that enterprises can trust for mission-critical workloads.

## 📈 Metric Definition
Success = A user can programmatically call `ManagementFactory.getMemoryMXBean().getHeapMemoryUsage()` and retrieve accurate bytes used/committed, and use `ThreadMXBean` to successfully detect a deadlocked thread cycle.

## 🔍 Gap Analysis
- **Current State:** Duke can run basic workloads, manage threads (Phase 28), and perform garbage collection, but all of this is invisible to the running application and external operators.
- **Market/Standard Lib:** Standard JVMs offer robust `java.lang.management` APIs and expose JMX (Java Management Extensions) endpoints out-of-the-box for tools like VisualVM.
- **The Gap:** Duke needs native bridges that expose the internal VM state (Heap size, GC counters, Thread states) safely to the Java layer.

## ✅ Acceptance Criteria
- Must implement the native bridges required by `java.lang.management.ManagementFactory`.
- Must provide `MemoryMXBean` exposing accurate heap and non-heap memory usage metrics.
- Must provide `ThreadMXBean` exposing thread counts, thread states, and basic deadlock detection logic.
- Must not introduce significant performance overhead when these metrics are not actively being queried.

## 🚫 Out of Scope
- Full JMX/RMI remote agent (exposing metrics over the network is a Phase 2 item; local JVM API access is Phase 1).
- Advanced flight recording (e.g., JFR).
- Setting/tuning JVM flags at runtime via MXBeans (read-only for now).
