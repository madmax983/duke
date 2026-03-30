# 🔭 Vantage: Spec for Java Debug Wire Protocol (JDWP) Support

## 👤 User Story
"As a Software Engineer debugging my Java application running on Duke, I want the VM to expose a JDWP-compliant debugging port, so that I can attach my standard IDE (IntelliJ, Eclipse, VS Code) to step through code, inspect variables, and evaluate expressions at runtime."

## ❓ The "So What?"
What business problem does this solve?
Developing and maintaining applications requires deep visibility into runtime state. Currently, if an application running on Duke encounters an unexpected state or logical error, developers must resort to primitive techniques like `System.out.println` or interpreting telemetry data. Without a debugger, troubleshooting complex business logic or resolving integration issues becomes prohibitively time-consuming. Supporting JDWP allows Duke to integrate seamlessly into existing developer workflows and tooling, proving that Duke is a serious, developer-friendly platform rather than just an opaque runtime black box. It dramatically lowers the barrier to entry and debugging cost for enterprise users.

## 📈 Metric Definition
Success = A user can start Duke with a JDWP agent enabled (e.g., `-agentlib:jdwp=transport=dt_socket,server=y,suspend=y,address=5005`), successfully attach a standard IDE debugger (like IntelliJ IDEA) to port 5005, hit a breakpoint in a Java method, inspect local primitive and object variables, and resume execution without the VM crashing or deadlocking.

## 🔍 Gap Analysis
- **Current State:** Duke executes bytecode but has no built-in mechanism to pause execution threads, inspect the heap, or communicate with external debugging clients via a standardized protocol.
- **Market/Standard Lib:** Standard JVMs (like HotSpot) implement the JVM Tool Interface (JVMTI) and provide a JDWP agent that translates JDWP wire commands into internal VM inspections. Modern IDEs rely exclusively on JDWP to interface with Java runtimes.
- **The Gap:** We need a networking layer to handle the JDWP binary protocol, an event management system to handle breakpoints/stepping, and safe hooks into the Duke interpreter's execution loop and memory heap to inspect/modify state concurrently.

## ✅ Acceptance Criteria
- Must implement the core JDWP handshake and basic command sets required for IDE attachment (VirtualMachine, ReferenceType, Method, EventRequest).
- Must support setting and hitting line number breakpoints (`EventRequest.Set` for `BREAKPOINT`).
- Must support suspending and resuming individual threads and the entire VM (`VirtualMachine.Suspend`, `VirtualMachine.Resume`).
- Must support retrieving the call stack (`ThreadReference.Frames`).
- Must support inspecting local variables (`StackFrame.GetValues`) and object fields (`ObjectReference.GetValues`).
- Must support stepping over and into methods (`EventRequest.Set` for `STEP`).

## 🚫 Out of Scope
- Advanced JDWP features like HotSwap (redefining classes at runtime).
- Evaluating complex arbitrary Java expressions via JDWP (this is usually handled by the IDE compiling a snippet and loading it, which relies on HotSwap or complex classloading out of scope for the initial JDWP implementation).
- JVMTI support (focus solely on the JDWP wire protocol directly within Duke for now, bypassing a C-level JVMTI interface).
