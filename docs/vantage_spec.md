# 🔭 Vantage: Spec for JDWP Debugger Integration

## 👤 User Story
As a Java Developer, I want to attach a standard IDE (like IntelliJ IDEA or Eclipse) to the Duke JVM via JDWP, so that I can inspect state and step through my application code just like I do on HotSpot.

## ❓ So What? (Business Problem)
Duke is a fast, safe JVM, but without observability tooling, it's essentially a black box. No one will use an experimental JVM in production or development if they can't debug their code when things go wrong. Supporting standard JDWP means zero-friction adoption for developers—they can use their existing tools.

## 🎯 Metric Definition
- **Success:** IntelliJ IDEA can connect to Duke via JDWP, set a breakpoint, and pause execution, with a connection overhead latency of `< 100ms`.
- **Engagement:** 100% compatibility with basic suspend/resume/step commands for standard Java tooling.

## 🔍 Gap Analysis
- **Current State:** Duke has internal `duke-telemetry` for JVM developers, but no standard Java debugger interface (JVMTI or JDWP) for end-users. It is completely opaque to standard Java IDEs.
- **Market (HotSpot/GraalVM):** Provide robust, built-in JDWP agents out of the box, which is a non-negotiable feature for serious Java development.

## ✅ Acceptance Criteria
- Must implement the Java Debug Wire Protocol (JDWP) core handshake.
- Must support connecting via socket transport (`dt_socket`).
- Must support suspending all threads, setting line-number breakpoints, and reading local variables from the current `Frame`.
- Must handle connection drops gracefully without crashing the JVM.

## 🚫 Out of Scope
- Advanced class redefinition / Hotswap.
- JVMTI native agent support (C/C++ agents) - focusing only on the wire protocol (JDWP) for now.
- Evaluation of complex arbitrary expressions during debug sessions.
