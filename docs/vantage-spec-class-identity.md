# 🔭 Vantage: Spec for Class Identity & Loader-Qualified Dedup

## 👤 User Story
"As a Backend Developer running a Spring Boot application, I want the JVM to correctly isolate and deduplicate classes loaded by different classloaders, so that complex applications can resolve classes unambiguously without encountering `ambiguous class name` or `ClassCastException` errors."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke fails to boot standard Spring Boot applications further into the context bootstrapping due to a deterministic class-identity blocker. The application registers the same class (`ApplicationListener`) under two loader-suffixed keys. A bare-name lookup finds both and cannot disambiguate, throwing an `ambiguous class name` error. Additionally, assignability checks miss because runtime keys are loader-qualified while checkcast targets resolve to the bare key, causing a `ClassCastException` (e.g., for `ConcurrentReferenceHashMap$SoftEntryReference`). By deduplicating loader-qualified class-keys, we unblock the next major step in running real-world enterprise frameworks on Duke.

## 📈 Metric Definition
Success = The Spring Boot app fixture bypasses the `ambiguous class name` (for `ApplicationListener`) and the `java exception: java/lang/reflect/InvocationTargetException` (wrapping a `ClassCastException`) blockers.

## 🔍 Gap Analysis
- **Current State:** Duke registers the SAME class under two loader-suffixed keys. When a bare-name lookup occurs, it finds both and cannot disambiguate. Furthermore, `is_assignable_from` misses because a referent's runtime key is loader-qualified while the checkcast target resolves to the bare key.
- **Market/Standard Lib:** Standard JVMs support complex classloader hierarchies (like Spring's `JarLauncher`) without ambiguous name resolution.
- **The Gap:** Duke needs to correctly deduplicate loader-qualified class-keys and handle assignability checks without mismatching bare keys and loader-qualified keys.

## ✅ Acceptance Criteria
- Must deduplicate loader-qualified class-keys so bare-name lookups can disambiguate correctly.
- Must eliminate the `ambiguous class name` error for `ApplicationListener`.
- Must resolve the `ClassCastException` (wrapped in `InvocationTargetException`) caused by assignability misses between loader-qualified and bare keys.

## 🚫 Out of Scope
- Resolving LambdaMetafactory or Array-Class issues.
