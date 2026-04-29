# 🔭 Vantage: Spec for IntStream.mapToDouble

## 👤 User Story
"As a Data Analyst or Backend Engineer using Duke, I want to map integer streams to double streams using `IntStream.mapToDouble`, so that I can compute precise statistical averages and numeric aggregations without manual object boxing overhead."

## ❓ The "So What?"
What business problem does this solve?
While Duke JVM has introduced `DoubleStream` and `LongStream.mapToDouble`, the standard `IntStream.mapToDouble` is still unimplemented. Java applications frequently use `IntStream` generated from loops (`IntStream.range`) or string length aggregations to perform statistical calculations. The lack of `mapToDouble` forces engineers to cast elements and box them into `Double` objects, resulting in massive garbage collection spikes, increased memory footprint, and slower execution time in data pipelines. Completing this primitive stream bridge ensures compatibility with standard Java 8 analytics workloads and maximizes CPU efficiency.

## 📈 Metric Definition
Success = A Java program can execute `IntStream.range(0, 100).mapToDouble(x -> x * 1.5).sum()` without triggering a native method missing error, and the garbage collection allocation overhead for intermediate elements is zero bytes.

## 🔍 Gap Analysis
- **Current State:** Duke has implemented `LongStream.mapToDouble` and basic `DoubleStream` features, but `native_int_stream_map_to_double` is missing from the interpreter's native method implementations.
- **Market/Standard Lib:** Standard JVMs (HotSpot, OpenJ9) support seamless primitive stream transformations without allocating intermediate boxed objects.
- **The Gap:** We need to implement the JVM native bridge `IntStream.mapToDouble(IntToDoubleFunction)` to invoke the user's lambda and collect the results into a `DoubleStream` representation.

## ✅ Acceptance Criteria
- Must implement `IntStream.mapToDouble(IntToDoubleFunction)` returning a valid `DoubleStream`.
- Must successfully execute the `applyAsDouble(int)` method on the provided lambda for each element.
- Must execute without boxing intermediate `double` values into `java.lang.Double` objects.
- Must handle empty streams gracefully, returning an empty `DoubleStream`.

## 🚫 Out of Scope
- Parallel execution (`IntStream.parallel().mapToDouble(...)`) is deferred to Phase 2.
- SIMD auto-vectorization.
