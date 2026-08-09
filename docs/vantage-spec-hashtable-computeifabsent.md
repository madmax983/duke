# 🔭 Vantage: Spec for Hashtable.computeIfAbsent

## 👤 User Story
"As a Java Framework Developer running on Duke, I want `java.util.Hashtable.computeIfAbsent` to be implemented, so that my applications can use default Map methods to lazily compute and store values without throwing a `NoSuchMethodError`."

## ❓ The "So What?"
What business problem does this solve?
Modern Java code and legacy logging frameworks (like Apache Commons Logging's `LogFactoryImpl`, used pervasively in Spring Boot) rely on `Hashtable.computeIfAbsent` to lazily cache and initialize instances in a thread-safe manner. Without this method, these frameworks crash during initialization when attempting to fallback to legacy loggers. Implementing this unlocks the ability to run critical standard logging frameworks and ensures compatibility with standard Java `Map` interface default method usage on legacy collections.

## 📈 Metric Definition
Success = `Hashtable.computeIfAbsent` executes successfully without throwing a `NoSuchMethodError`, executing the provided `Function` if the key is missing, and the Spring Boot ladder test proceeds past the `LogFactoryImpl` initialization wall.

## 🔍 Gap Analysis
- **Current State:** Duke has a synthetic `java.util.Hashtable` but it does not implement the `computeIfAbsent(Object,Function)` default method added in Java 8.
- **Market/Standard Lib:** Java 8+ added default methods to the `Map` interface, which were specifically overridden in `Hashtable` to provide synchronization. Legacy enterprise apps use it heavily.
- **The Gap:** We need to implement the native bridge `Hashtable.computeIfAbsent(Object, Function)` to conditionally invoke the lambda callback and store the result if the key is absent.

## ✅ Acceptance Criteria
- Must implement `Hashtable.computeIfAbsent(Object, Function)` returning the computed value or existing value.
- Must execute the `apply` method on the provided lambda/Function if the key is not present.
- Must store the computed value into the `Hashtable` if the computed value is non-null.
- Must not throw `NoSuchMethodError` when invoked by `LogFactoryImpl` fallback.

## 🚫 Out of Scope
- Fine-grained lock implementation for thread-safety (assume basic sequential consistency for this phase).
- Other unimplemented default `Map` methods on `Hashtable` (e.g., `merge`, `replace`).
