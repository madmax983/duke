# 🔭 Vantage: Spec for OSS JAR Smoke Widening

## 👤 User Story
"As a Java Developer evaluating Duke, I want Duke to successfully execute common real-world libraries like `gson` and `commons-lang3`, so that I can trust it to run my everyday dependencies without failing on missing core JVM features."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke's OSS JAR smoke harness only successfully runs a narrow slice of the ecosystem (`slf4j`). Modern Java applications heavily rely on standard utility and serialization libraries like `gson` and `commons-lang3`. Failing to run these libraries due to missing standard JVM capabilities (like specific string formatting bugs, missing Unicode regex support, and incomplete reflection APIs) severely limits Duke's utility. Widening compatibility to these libraries proves Duke's correctness and readiness for a broader set of real-world workloads, moving it closer to being a drop-in JVM replacement.

## 📈 Metric Definition
Success = The `oss_jar_smoke` tests for both `gson-2.11.0.jar` and `commons-lang3-3.17.0.jar` can be executed end-to-end (`cargo test -p duke-interpreter --test oss_jar_smoke`) without throwing exceptions related to unsupported natives, invalid memory references, or missing standard library capabilities.

## 🔍 Gap Analysis
- **Current State:**
  - `gson` fails during `JsonWriter.<clinit>` due to a bug in `java/lang/String.format` returning an invalid heap reference for `%04x`. Static analysis infers missing reflection capabilities (`Field.getModifiers`, `Field.getGenericType`, and `Unsafe.allocateInstance`) will be the next blockers.
  - `commons-lang3` fails during `StringUtils.<clinit>` because the regex engine doesn't support the `\p{InCombiningDiacriticalMarks}` Unicode block property. Static analysis infers missing reflection capabilities (`Array.newInstance`, `Array.getLength`, `Array.set`) will be the next blockers.
- **Market/Standard Lib:** Standard JVMs fully implement `String.format`, complete Unicode regex properties, and comprehensive reflection APIs (`java.lang.reflect.Field`, `java.lang.reflect.Array`, and `sun.misc.Unsafe`).
- **The Gap:** Duke needs to fix the existing `String.format` bug, add Unicode block support to the regex engine, and implement the missing reflection native methods for `Field` and `Array` (and potentially `Unsafe`).

## ✅ Acceptance Criteria
- Must successfully execute the `gson` happy path: `new Gson().toJson(pojo)` and `gson.fromJson(json, Pojo.class)` without errors.
- Must successfully execute the `commons-lang3` happy path: `StringUtils.join`, `StringUtils.capitalize`, `ArrayUtils.add`, `ArrayUtils.contains`, and `ArrayUtils.toObject` without errors.
- Must fix the `java/lang/String.format` bug that currently returns an `InvalidRef` for `%04x`.
- Must implement support for the `\p{InCombiningDiacriticalMarks}` Unicode block in the regex engine (`Pattern.compile`).
- Must implement missing `java/lang/reflect/Field` native methods (e.g., `getModifiers()`, `getGenericType()`).
- Must implement missing `java/lang/reflect/Array` native methods (e.g., `newInstance()`, `getLength()`, `set()`).

## 🚫 Out of Scope
- Complete implementation of all `sun.misc.Unsafe` methods (only what is strictly required for `gson`'s `fromJson` fallback is needed, if reached).
- Ensuring compatibility with *all* features of `gson` and `commons-lang3` beyond the defined happy paths.
- Performance optimization of the newly added reflection or regex capabilities.
