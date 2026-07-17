# 🔭 Vantage: Spec for ArrayDeque Initial Capacity Constructor

## 👤 User Story
"As a Backend Developer running my code on Duke, I want the VM to natively support the `java/util/ArrayDeque.<init>(I)V` constructor, so that my applications can initialize double-ended queues with a specific capacity without crashing."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke halts execution when it encounters `java/util/ArrayDeque.<init>(I)V`. This prevents many enterprise applications and frameworks—specifically Spring Boot—from successfully booting, as they rely on this constructor internally for collection initialization. By supporting this constructor, we unblock the Spring Boot Fat JAR boot process, allowing users to deploy standard modern enterprise microservices on Duke.

## 📈 Metric Definition
Success = A user can run a Spring Boot Fat JAR (or any application calling `new ArrayDeque(capacity)`) and the application proceeds past the constructor without encountering a "method not found" runtime error for `java/util/ArrayDeque.<init>(I)V`.

## 🔍 Gap Analysis
- **Current State:** Duke supports `ArrayDeque.<init>()V` (the no-args constructor) but throws an error when `java/util/ArrayDeque.<init>(I)V` is invoked. The Spring Boot real app canary is currently pinned at this missing capability.
- **Market/Standard Lib:** The standard Java `ArrayDeque` provides an `ArrayDeque(int numElements)` constructor to allocate an initial backing array of a sufficient size to hold the specified number of elements.
- **The Gap:** We need to register and implement the native method for `java/util/ArrayDeque.<init>(I)V` in `crates/duke-interpreter/src/stdlib.rs` and `crates/duke-interpreter/src/native/java_util.rs`.

## ✅ Acceptance Criteria
- Must register the `java/util/ArrayDeque.<init>(I)V` method in the standard library registry.
- Must implement the native logic for `java/util/ArrayDeque.<init>(I)V`.
- The native implementation should initialize the `ArrayDeque` correctly (even if it currently delegates to `native_arraylist_init_with_capacity` or simply initializes the size to 0 and ignores capacity, mimicking the existing `ArrayList.<init>(I)V` behavior).
- The `APP_BLOCKER` pin in `duke/tests/spring_boot_real_app.rs` must clear this step and advance to the next missing capability or successfully boot.

## 🚫 Out of Scope
- Re-architecting the `ArrayDeque` memory layout.
- Implementing other missing `ArrayDeque` methods not currently blocking the Spring Boot boot sequence.
- Implementing `HttpURLConnection` (already handled in previous efforts).
