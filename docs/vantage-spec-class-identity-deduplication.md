# 🔭 Vantage: Spec for Class Identity & Loader-Key Deduplication

## 👤 User Story
"As a Platform Engineer running complex enterprise applications, I want my runtime to correctly deduplicate and identify classes loaded by different class loaders, so that I don't encounter spurious `ambiguous class name` errors or `ClassCastException`s when evaluating `instanceof` or `checkcast` on reflectively loaded classes."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke fails to boot large frameworks like Spring Boot because it registers the same class under multiple loader-qualified keys (e.g., `org/springframework/context/ApplicationListener matches [..\0, ..\0]`). This causes deterministic crashes when looking up classes by bare name or when comparing class identities dynamically. By implementing correct loader-key deduplication and class identity checks, we unlock the ability to run real-world, enterprise-grade Java applications (fat JARs) that heavily rely on complex class-loader hierarchies.

## 📈 Metric Definition
Success = The Spring Boot test advances past the `ambiguous class name` and `java exception: java/lang/reflect/InvocationTargetException` (wrapping ClassCastException) walls.

## 🔍 Gap Analysis
- **Current State:** Duke registers classes with loader-qualified names but fails to disambiguate or correctly compare them in certain reflection and assignability checks.
- **Market/Standard Lib:** The standard JVM maintains a rigorous class identity model where two classes are identical if and only if they have the same fully qualified name AND were loaded by the same defining class loader.
- **The Gap:** We need to normalize class key comparisons across the reflection API and interpreter instructions (`instanceof`, `checkcast`) so that the VM correctly understands when two differently-loaded references refer to the same or different runtime classes.

## ✅ Acceptance Criteria
- Must deduplicate classes correctly when registered by a class loader.
- Must accurately perform class identity checks (`instanceof`, `checkcast`, `isAssignableFrom`) taking loader qualifiers into account properly or stripping them when comparing bare names.
- Must clear the `ambiguous class name` wall for `ApplicationListener`.
- Must clear the `ClassCastException` wrapped in `InvocationTargetException` for `ConcurrentReferenceHashMap$SoftEn...`.

## 🚫 Out of Scope
- Implementing custom ClassLoaders from scratch.
- Fixing the subsequent LambdaMetafactory / invokedynamic wall.
