# 🔭 Vantage: Spec for Double.longBitsToDouble

## 👤 User Story
"As a Systems Engineer writing numerical or serialization code on Duke, I want the VM to support `Double.longBitsToDouble(long)`, so that I can deserialize binary data streams into floating-point numbers without losing precision or dealing with corrupt bits."

## ❓ The "So What?"
What business problem does this solve?
Many high-performance binary formats, network protocols, and standard Java library internals (like `ObjectInputStream` and `ByteBuffer`) read raw 64-bit integers from the wire or memory and need to safely reinterpret them as `double` floating-point values. Without this low-level intrinsic conversion, any application attempting to read binary floating-point data will crash due to a missing native method. Implementing this unblocks basic binary data processing and networking protocols.

## 📈 Metric Definition
Success = A Java program can execute `Double.longBitsToDouble(0x400921FB54442D18L)` and successfully return `3.141592653589793` without throwing a native method missing error.

## 🔍 Gap Analysis
- **Current State:** Duke has basic mathematical operations, but `Double.longBitsToDouble(long)` is currently a stubbed `TODO` native method tracked in the interpreter (`crates/duke-interpreter/src/stdlib.rs`).
- **Market/Standard Lib:** Standard JVMs (HotSpot, OpenJ9) implement this directly via fast CPU-level bit-casting operations.
- **The Gap:** We need to provide the JVM native bridge `Double.longBitsToDouble(long)` to seamlessly translate a 64-bit integer into its exact `double` floating-point representation using Rust's `f64::from_bits`.

## ✅ Acceptance Criteria
- Must implement the native method `java.lang.Double.longBitsToDouble(long)` returning a `double`.
- Must successfully convert the 64-bit integer bit pattern into the corresponding floating-point value.
- Must correctly handle IEEE 754 edge cases such as NaN, positive infinity, and negative infinity.

## 🚫 Out of Scope
- Implementation of the inverse operation `Double.doubleToRawLongBits` (if not already supported).
- Optimizing this operation for auto-vectorized SIMD arrays.
