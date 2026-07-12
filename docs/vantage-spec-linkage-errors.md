# 🔭 Vantage: Spec for LinkageError Handling (NoClassDefFoundError)

## 👤 User Story
"As a Framework Developer running my code on Duke (e.g., using Spring Boot or commons-logging), I want class resolution failures during runtime execution (like missing logging backend classes) to throw a catchable `NoClassDefFoundError` rather than causing a fatal VM crash, so that my application's fallback and capability-probing logic can execute normally."

## ❓ The "So What?"
What business problem does this solve?
Many standard Java libraries and frameworks (like Spring Boot, SLF4J, and Apache Commons Logging) use "soft" dependencies. They deliberately attempt to load or link optional classes (like specific logging backends such as Log4j or Logback) and rely on catching `LinkageError` or `NoClassDefFoundError` to fall back to a default implementation if the class is absent. Currently, Duke treats runtime class resolution failures (e.g., during `<clinit>` execution or method linkage) as fatal VM errors, immediately terminating the application. Without translating these resolution failures into standard, catchable Java exceptions, Duke is unable to run robust enterprise applications that depend on dynamic capability probing.

## 📈 Metric Definition
Success = When the runtime fails to resolve a class (e.g., `org/apache/logging/log4j/MarkerManager`) during bytecode execution (such as in a `<clinit>` method), Duke creates and throws a standard Java `java.lang.NoClassDefFoundError` (which extends `LinkageError`) within the interpreter. The Spring Boot ladder fixture should advance past its current `class not found` fatal error and execute its fallback logic.

## 🔍 Gap Analysis
- **Current State:** The interpreter surfaces class-resolution failures (like `ClassNotFound`) during execution as fatal Rust `Err(...)` returns, which propagate up and terminate the VM. Duke currently lacks synthetic representations of `NoClassDefFoundError` and its parent `LinkageError`.
- **Market/Standard Lib:** A standard JVM raises a catchable `NoClassDefFoundError` (a subclass of `LinkageError`) when class resolution fails during linkage or execution (e.g., missing dependencies from the classpath).
- **The Gap:** We need to implement synthetic classes for `NoClassDefFoundError` (and its parent `LinkageError`), and update the interpreter's class-resolution and method-linkage paths to catch internal resolution failures and translate them into a thrown Java exception instead of a fatal error.

## ✅ Acceptance Criteria
- Must include a synthetic class registration for `java.lang.NoClassDefFoundError` and `java.lang.LinkageError`.
- Must translate class resolution failures during bytecode execution (e.g., missing classes during method dispatch, field access, or `<clinit>`) into a thrown `NoClassDefFoundError` within the Java thread.
- The thrown `NoClassDefFoundError` must be catchable by standard Java `try-catch` blocks designed to catch `Throwable`, `Error`, `LinkageError`, or `NoClassDefFoundError`.
- The `duke-spring-boot-ladder-3.5.12.jar` fixture must advance past the `MarkerManager` class resolution failure, allowing `commons-logging` to catch the error and continue initialization.

## 🚫 Out of Scope
- Throwing `ClassNotFoundException` (which is typically thrown by explicit reflection `Class.forName` or `ClassLoader.loadClass`, not by implicit JVM linkage resolution).
- Supporting every subtype of `LinkageError` (e.g., `IncompatibleClassChangeError`, `NoSuchMethodError`) at this time; focus strictly on `NoClassDefFoundError`.
