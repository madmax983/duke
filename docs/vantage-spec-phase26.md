# 🔭 Vantage: Spec for Native Callback Mechanism + `Collections.sort`

## 👤 User Story
"As a Java Developer running my code on Duke, I want to be able to sort standard collections (like `ArrayList`) using `Collections.sort`, so that I can organize data efficiently without writing custom sorting algorithms for standard types."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke lacks the ability to execute the heavily-used `Collections.sort()` method on standard library data structures. Developers rely on these utility methods to write concise, standard Java. Without it, Duke is incompatible with a vast amount of existing Java 8+ code that assumes `Collections.sort` is available and performs natural ordering. Providing this utility reduces friction for adoption and increases the utility of the VM for real-world applications. Complexity here (the callback mechanism) is an investment to support a wide range of standard library features that rely on invoking Java methods from native code.

## 📈 Metric Definition
Success = `Collections.sort()` executes correctly on a `List` of standard Comparable types (`Integer`, `String`, `Long`, `Double`) containing at least 5 elements, producing the naturally ordered sequence.

## 🔍 Gap Analysis
- **Current State:** Duke's native handlers cannot invoke Java bytecode methods. `Collections.sort` requires calling the `compareTo` method on elements, which are Java methods.
- **Market/Standard Lib:** Java 8+ compiles `Collections.sort(list)` to `list.sort(null)`. The standard library relies on the VM to dispatch this `sort` method and use the elements' `compareTo` implementations.
- **The Gap:** We need a bridge for native code (`ArrayList.sort`) to call back into the interpreter (`compareTo`). We also need the `compareTo` implementations for the core boxed primitives.

## ✅ Acceptance Criteria
- Must handle natural ordering (a `null` Comparator) for standard types.
- Must correctly sort `ArrayList`s containing `Integer` objects.
- Must correctly sort `ArrayList`s containing `String` objects.
- Must implement `compareTo` natives for `Integer`, `String`, `Long`, and `Double` following the standard Java contract (-1, 0, 1).
- Must handle sorting gracefully when elements are `null` (by throwing standard exceptions if required by `compareTo`, or handling it per the sort implementation).
- Must handle sorting empty lists or lists with a single element without error.

## 🚫 Out of Scope
- Custom `Comparator` implementations (i.e., non-null Comparators passed to `sort`).
- Highly optimized sorting algorithms (e.g., TimSort). A simple, verifiable `O(n^2)` insertion sort is acceptable for this phase.
