# 🔭 Vantage: Spec for Class Identity

## 👤 User Story
As a framework developer using dynamic class loading (like Spring Boot), I want the JVM to correctly disambiguate classes loaded by different class loaders, so that I can reliably perform reflection, type casting, and dependency injection without encountering ambiguous class names or `ClassCastException`s.

## ❓ So What?
What business problem does this solve?
Currently, our real-world Spring Boot application fails to boot due to a loader-qualified class-key dedup issue. The same class (`ApplicationListener`) is registered under two loader-suffixed keys, causing a bare-name lookup to fail with an `ambiguous class name` error. Additionally, instances (like `ConcurrentReferenceHashMap$SoftEntryReference`) fail `is_assignable_from` checks when the runtime key is loader-qualified but the checkcast target resolves to the bare key, resulting in a `ClassCastException` (wrapped in an `InvocationTargetException` by `main.invoke`). Resolving this unblocks the next major phase of the Spring Boot application boot process (dependency injection/ApplicationContext refresh).

## ✅ Acceptance Criteria
- Success = The Spring Boot app fixture advances past the `ambiguous class name` wall for `org/springframework/context/ApplicationListener`.
- Success = The Spring Boot app fixture does not encounter a `ClassCastException` related to loader-qualified keys (e.g., when casting `ConcurrentReferenceHashMap$SoftEntryReference`'s soft referent).
- Bare-name class lookups correctly disambiguate or resolve the appropriate loader-specific class context.
- Runtime assignability checks correctly evaluate identical classes loaded via different/loader-suffixed keys.

## 🚫 Out of Scope
- Full implementation of `ApplicationContext` refresh logic.
- Fixing lambda proxy resolution or LambdaMetafactory (covered in a separate lane).
- Addressing missing primitive array-class resolutions.
