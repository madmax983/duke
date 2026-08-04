# 🔭 Vantage: Spec for Classloader Isolation and Identity

## 👤 User Story
"As a Framework Developer, I want classes to be uniquely identified by their ClassLoader and Name pair, so that my application can safely run multiple isolated instances of the same class (like different versions of a library in a web container) without `ClassCastException` or `ambiguous class name` errors."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke's class registry identifies classes primarily by their bare name. When a modern framework like Spring Boot uses custom ClassLoaders (e.g., `org.springframework.boot.loader.launch.JarLauncher`) to load internal classes, those classes are registered under a loader-qualified key (e.g., `\0loader:154`). However, `is_assignable_from` checks and bare-name lookups fail to correctly disambiguate identical bare names across different loaders. This causes legitimate reflection calls to throw `InvocationTargetException` (wrapping a `ClassCastException`) or fail with `ambiguous class name`. Fixing this proves Duke can support complex, multi-module enterprise architectures that rely on ClassLoader isolation.

## 📈 Metric Definition
Success = The `spring_boot_app_surfaces_next_missing_capability_explicitly` test in `duke/tests/spring_boot_real_app.rs` clears the `ambiguous class name` and `java exception: java/lang/reflect/InvocationTargetException` (wrapping a `ClassCastException`) blockers, allowing the Spring Boot app boot process to proceed to the next frontier.

## 🔍 Gap Analysis
- **Current State:** Duke registers classes loaded by custom classloaders with a loader suffix, but fails to use this suffix consistently for identity and assignability checks.
- **Market/Standard Lib:** The JVM Specification (§5.3) dictates that a runtime class is determined by the tuple `(N, L)`, where `N` is the class name and `L` is the defining class loader.
- **The Gap:** We need to update Duke's internal class registry, lookup mechanisms, and assignability checks (`is_assignable_from`) to strictly enforce the `(N, L)` identity tuple.

## ✅ Acceptance Criteria
- Must identify a runtime class strictly by its `(Name, ClassLoader)` pair.
- `is_assignable_from` must correctly return `true` only if both the name and the defining classloader of the two classes match.
- Bare-name lookups must correctly resolve to the requesting context's defining classloader, avoiding `ambiguous class name` errors when multiple classloaders hold identically-named classes.

## 🚫 Out of Scope
- Implementing custom user-defined ClassLoaders that dynamically generate bytecode.
- Garbage collection of unloaded ClassLoaders and their classes (class unloading).
