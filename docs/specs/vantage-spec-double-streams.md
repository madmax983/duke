# 🔭 Vantage: Spec for Java 8 Double Streams (`java.util.stream.DoubleStream`)

## 👤 User Story
"As a Data Engineer or Scientist running my numerical workloads on Duke, I want to use `DoubleStream` and its associated mapping functions and collectors, so that I can perform high-performance functional data processing on floating-point data without paying the performance penalty of boxing `double` primitives into `Double` objects."

## ❓ The "So What?"
What business problem does this solve?
While Duke supports basic streams (`Stream<T>`) and integer streams (`IntStream`, `LongStream`), it currently lacks full support for `DoubleStream` and the various mapping functions that bridge object streams to double streams (e.g., `Stream.mapToDouble`, `LongStream.mapToDouble`, `Collectors.summingDouble`, etc.). In data processing, financial applications, and scientific computing, `double` is the standard type for precision math. Forcing these users to use `Stream<Double>` results in massive memory allocation overhead (boxing) and cache misses, rendering Duke uncompetitive for numerical analysis. Supporting `DoubleStream` completes the Java 8 primitive stream triad and enables high-performance numeric processing.

## 📈 Metric Definition
Success = A Java program running on Duke can successfully execute standard `DoubleStream` pipelines (such as `.mapToDouble(x -> ...).sum()`, `flatMapToDouble()`, and `.collect(Collectors.summingDouble(...))`) correctly producing the expected floating-point aggregate results.

## 🔍 Gap Analysis
- **Current State:** Duke has unimplemented native methods for `Stream.mapToDouble`, `LongStream.mapToDouble`, `Stream.flatMapToDouble`, `Collectors.summingDouble`, and `Collectors.averagingDouble`.
- **Market/Standard Lib:** Standard Java 8 introduced `DoubleStream` alongside `IntStream` and `LongStream`. The standard library implements these using specialized spliterators and stream pipelines optimized for `double` arrays and elements.
- **The Gap:** We need to implement the native JVM bridges for the missing stream mapping functions and collectors that deal with `DoubleStream` and `ToDoubleFunction`.

## ✅ Acceptance Criteria
- Must implement `Stream.mapToDouble(ToDoubleFunction)` returning a valid `DoubleStream`.
- Must implement `LongStream.mapToDouble(LongToDoubleFunction)` returning a valid `DoubleStream`.
- Must implement `Stream.flatMapToDouble(Function<T, DoubleStream>)` returning a flattened `DoubleStream`.
- Must implement `Collectors.summingDouble(ToDoubleFunction)` returning a Collector that computes the sum correctly.
- Must implement `Collectors.averagingDouble(ToDoubleFunction)` returning a Collector that computes the average correctly.
- Tests from Phase 49, Phase 55, and Phase 59 covering these capabilities must pass.

## 🚫 Out of Scope
- Parallel stream processing execution (`DoubleStream.parallel()`).
- Auto-vectorization of DoubleStream pipelines at the native level (focus is strictly on functional correctness and baseline JVM compatibility).
