# 🔭 Vantage: Spec for Just-In-Time (JIT) Compilation

## 👤 User Story
"As a Performance Engineer, I want the Duke VM to dynamically compile hot bytecode into native machine code, so that long-running applications achieve near-native execution speeds."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke is a pure interpreter. While this is simple and easy to maintain, it incurs a massive performance overhead for every instruction executed. For any serious computational workload or long-running server application, the pure interpreter speed is uncompetitive with standard JVMs. Implementing a JIT compiler allows Duke to bridge the performance gap by optimizing frequently executed code paths (hot spots), making Duke viable for performance-sensitive, production environments.

## 📈 Metric Definition
Success = Executing a CPU-intensive benchmark (e.g., matrix multiplication or Mandelbrot generation) completes at least 10x faster when JIT is enabled compared to the pure interpreter.

## 🔍 Gap Analysis
- **Current State:** Duke evaluates all instructions via a massive `match` statement in the `execute` and `run_execution` loops, incurring dispatch overhead and preventing cross-instruction optimizations.
- **Market/Standard Lib:** Modern JVMs (HotSpot, OpenJ9) employ tiered compilation, starting with an interpreter and gradually compiling "hot" methods (C1/C2 compilers) into optimized machine code using techniques like inlining and loop unrolling.
- **The Gap:** We need a way to identify hot methods, translate their bytecode into an Intermediate Representation (IR), perform basic optimizations, and generate executable native machine code for the target architecture.

## ✅ Acceptance Criteria
- Must introduce a profiling mechanism to identify "hot" methods (e.g., via invocation counters or backedge counters).
- Must include a compiler backend capable of generating executable machine code (e.g., using Cranelift or LLVM) for at least one architecture (x86_64 or AArch64).
- Must transparently fall back to the interpreter for unsupported features or cold code.
- Must not degrade the startup time of short-lived applications significantly.

## 🚫 Out of Scope
- Advanced optimizations like escape analysis, auto-vectorization, or aggressive deoptimization (Phase 1 focus on basic tier-1 compilation).
- Support for 32-bit architectures or exotic instruction sets.
