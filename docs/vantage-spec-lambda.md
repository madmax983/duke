# 🔭 Vantage: Spec for Lambda Expressions & Method Handles

## 👤 User Story
"As a Modern Java Developer running my code on Duke, I want to use Lambda expressions and Streams, so that I can write concise, functional-style code that is standard in Java 8 and above."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke lacks support for `invokedynamic` (specifically the `LambdaMetafactory`), which is the underlying mechanism used by the Java compiler to translate lambda expressions (`() -> {}`) and method references (`String::length`) into executable bytecode. Without this, practically all modern Java code (Java 8+) fails to load or execute. Supporting Lambdas is non-negotiable for running any contemporary enterprise codebase, including Spring Boot, modern data processing pipelines, and even standard library collections. This feature upgrades Duke from a legacy JVM to a modern Java runtime.

## 📈 Metric Definition
Success = A user can compile and execute a Java program containing a lambda expression (e.g., `Runnable r = () -> System.out.println("Hello"); r.run();`) and a Stream API chain (e.g., `list.stream().filter(x -> x > 5).count()`), and the VM successfully resolves and executes the lambdas without throwing `BootstrapMethodError` or panicking.

## 🔍 Gap Analysis
- **Current State:** Duke can execute basic bytecode but throws an error or panics when encountering the `invokedynamic` opcode, specifically when it delegates to `java.lang.invoke.LambdaMetafactory`.
- **Market/Standard Lib:** The standard JVM uses `invokedynamic` linked to the `LambdaMetafactory` to dynamically generate synthetic classes at runtime that implement the target functional interface. This heavily relies on `java.lang.invoke.MethodHandle` and `MethodType`.
- **The Gap:** We need to implement the `invokedynamic` bytecode instruction, support resolution of Bootstrap Methods (BSM) in the constant pool, and provide the native JVM support required by `LambdaMetafactory` to spin up functional interface implementations dynamically.

## ✅ Acceptance Criteria
- Must implement the `invokedynamic` bytecode instruction execution loop.
- Must support resolving Bootstrap Methods from the ClassFile's constant pool.
- Must provide sufficient native support for `java.lang.invoke.MethodHandle`, `MethodType`, and related classes to allow the `LambdaMetafactory` to operate.
- Must correctly execute standard stateless lambda expressions (e.g., `() -> println("test")`).
- Must correctly execute capturing lambda expressions (e.g., closing over local variables like `(x) -> x + y`).
- Must correctly execute method references (e.g., `System.out::println`).

## 🚫 Out of Scope
- Dynamic language support (e.g., JRuby, Nashorn). The focus is entirely on Java's `LambdaMetafactory` usage of `invokedynamic`.
- Highly optimized custom CallSite linkage caching. A slow, naive resolution path is acceptable for the initial implementation as long as it is correct.
- `StringConcatFactory` (Java 9+ string concatenation via `invokedynamic`), though it may naturally fall out of a robust `invokedynamic` implementation, the focus is Lambdas.
