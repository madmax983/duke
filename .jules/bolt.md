**[JImage Index Map Preallocation]
**Learning:** [HashMap::new() for large, known-size collections causes severe reallocation overhead during startup, especially when parsing files with 30,000+ entries like the JDK jimage file.]
**Action:** [Always use `HashMap::with_capacity(capacity)` when the final size of the collection is already known from headers (e.g., `resource_count`).]
**[Optimize Parser Allocations]
**Learning:** Iterator chains like `.map().collect()` on complex structures returning `Result<Vec<T>, E>` can obscure exact allocations, even when the iterator length is known.
**Action:** Replace these chains with explicit `Vec::with_capacity(count)` and `for` loops in hot parsing paths (like `duke-classfile/src/parser.rs`) to ensure zero intermediate allocations and explicit sizing.

**Refactor `.iter().cloned().collect::<Vec<_>>()` to `.clone()`**
**Learning:** `.iter().cloned().collect::<Vec<_>>()` is an inefficient anti-pattern for types that already implement `Clone`. It wastes overhead on iterators and potentially multiple heap allocations, where `.clone()` provides a clean, single-allocation copy of the vector.
**Action:** Always prefer `.clone()` directly on collections instead of an iterator chain when duplicating elements.
