# 🔭 Vantage: Spec for Phase 79 (Immutable Collections & Objects)

## 👤 User Story
"As a Java Developer running my code on Duke, I want to use standard immutable collection factory methods (`List.of`, `Set.of`, `Map.of`) and `java.util.Objects` utilities, so that I can write concise, modern, and safe code without relying on verbose initialization patterns."

## ❓ The "So What?"
What business problem does this solve?
Modern Java (9+) heavily utilizes factory methods like `List.of()` and `Map.of()` to create unmodifiable collections. Additionally, `java.util.Objects` (like `requireNonNull` and `equals`) is ubiquitous for null-safety and equality checks. Without these, developers must rewrite standard code to run on Duke. Supporting these standard APIs ensures broader compatibility, reduces boilerplate, and makes the VM a viable target for modern Java libraries.

## 📈 Metric Definition
Success = The standard immutable collection factory methods (`List.of`, `Set.of`, `Map.of`) correctly create and return collections that behave properly (e.g., retrieval, `contains` checks) and `java.util.Objects` methods properly validate inputs. The `Phase79Test` suite must pass entirely.

## 🔍 Gap Analysis
- **Current State:** Duke lacks implementations for modern collection factory methods and standard utility methods in `java.util.Objects`, preventing modern Java code from executing successfully.
- **Market/Standard Lib:** Java 9 introduced these immutable collection factories, which return lightweight, unmodifiable implementations.
- **The Gap:** We need to implement native handlers or support standard bytecode execution for `List.of`, `Set.of`, `Map.of`, and `Objects.requireNonNull`/`Objects.equals`.

## ✅ Acceptance Criteria
- Must support `List.of()` creating an immutable list and handle methods like `get()` and `contains()`.
- Must support `Set.of()` creating an immutable set and handle `contains()`.
- Must support `Map.of()` creating an immutable map and handle `get()`.
- Must support `Objects.requireNonNull()` throwing a `NullPointerException` when passed `null`, and returning the object otherwise.
- Must support `Objects.equals()` correctly evaluating equality, including handling `null` references safely.
- The `Phase79Test` integration tests must all pass successfully.

## 🚫 Out of Scope
- Full enforcement of immutability (e.g., throwing `UnsupportedOperationException` on mutation attempts) is not strictly required if the tests do not demand it, though it is preferred.
- Supporting overloaded factory methods beyond the basic `of()` variations required by the tests.
