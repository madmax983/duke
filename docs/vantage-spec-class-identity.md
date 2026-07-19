# 🔭 Vantage: Spec for Class Identity and Loader Isolation

## 👤 User Story
"As a Java Framework Developer running my code on Duke, I want classes loaded by different classloaders to maintain correct identity and isolation, so that I can use complex deployment models (like Fat JARs, modules, or plugins) without runtime type collisions or spurious ClassCastExceptions."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke fails to properly distinguish between classes with the same name loaded by different classloaders. This leads to `ambiguous class name` errors when resolving classes (e.g., in Spring Boot's ApplicationListener) and spurious `ClassCastException`s when evaluating `instanceof` or `checkcast`. In modern Java (like Spring Boot), custom classloaders are ubiquitous. If Duke cannot guarantee standard Java class identity semantics, it cannot run any modern framework, limiting it to simple scripts.

## 📈 Metric Definition
Success = The `ambiguous class name` and `ClassCastException` errors in the Spring Boot real app test are eliminated, and Duke consistently passes the `is_assignable_from` check for classes across different valid classloader hierarchies without false negatives.

## 🔍 Gap Analysis
- **Current State:** Duke registers classes with loader-qualified keys (e.g., `Name\0loader:ID`), but lookups and assignability checks inconsistently mix bare names and loader-qualified keys, causing ambiguity and false negatives.
- **Market/Standard Lib:** The standard JVM explicitly defines a runtime class as the pair `(N, L)`. Two classes with the same name loaded by different classloaders are treated as distinct types.
- **The Gap:** We need to overhaul the class registry lookup and assignability checks to consistently respect loader-qualified keys, eliminating the `ambiguous class name` error on duplicate keys and ensuring `checkcast` resolves correctly.

## ✅ Acceptance Criteria
- Must eliminate the `ambiguous class name` error by properly deduplicating or disambiguating class lookups based on the active classloader.
- Must ensure that `is_assignable_from` correctly identifies identical classes even if they are queried using a mix of bare names and loader-qualified keys within the same loader hierarchy.
- Must not introduce regressions in standard class resolution for single-classloader applications.
- Must clear the current `ambiguous class name` and `InvocationTargetException` (wrapping `ClassCastException`) blockers in `duke/tests/spring_boot_real_app.rs`.

## 🚫 Out of Scope
- Complete implementation of Java 9+ Modules (JPMS) isolation.
- Support for custom user-defined ClassLoaders running complex dynamic byte-weaving unless they rely on standard ClassLoader delegation.
