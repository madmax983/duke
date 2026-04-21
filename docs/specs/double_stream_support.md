# Spec: Java Streams API - Double Primitive Support

👤 **User Story:** "As a data analyst or numerical computing engineer, I want the JVM to support `DoubleStream` methods (like `mapToDouble`), so that I can process floating-point datasets efficiently without manual boxing/unboxing overhead."

**"So What?" (Business Problem):**
Financial applications, scientific simulations, and machine learning data pipelines heavily rely on primitive `double` arrays. Lack of native `DoubleStream` support forces the use of boxed `Double` objects, resulting in massive GC pressure, memory bloat, and cache misses that degrade execution speed on large datasets.

**Metric Definition (Success):**
- Success = `Stream.mapToDouble` and `LongStream.mapToDouble` execute without triggering an `UnsatisfiedLinkError` or `TODO` panic in the interpreter.
- Success = GC allocation overhead for intermediate stream primitives is reduced to zero (0 bytes allocated per element) in benchmarks.

**Gap Analysis:**
Currently, the standard library support in Duke JVM relies heavily on `int` and `long` primitive streams (`IntStream`, `LongStream`). Any code trying to use `mapToDouble` crashes due to missing native method implementations. This prevents the execution of modern, standard Java applications that rely on numerical aggregations.

✅ **Acceptance Criteria:**
- Java applications using `Stream.mapToDouble` and `LongStream.mapToDouble` must execute successfully.
- The execution must correctly map `Integer` or `Long` values to primitive `double` values as dictated by the provided lambdas.
- All integration test fixtures targeting these methods must pass.

🚫 **Out of Scope:**
- Parallel stream processing execution (`DoubleStream.parallel()`).
- Optimizing JVM floating-point math intrinsics (reusing existing FP stack mechanics).