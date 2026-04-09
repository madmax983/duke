**[JImage Index Map Preallocation]
**Learning:** [HashMap::new() for large, known-size collections causes severe reallocation overhead during startup, especially when parsing files with 30,000+ entries like the JDK jimage file.]
**Action:** [Always use `HashMap::with_capacity(capacity)` when the final size of the collection is already known from headers (e.g., `resource_count`).]
**[Optimize Parser Allocations]
**Learning:** Iterator chains like `.map().collect()` on complex structures returning `Result<Vec<T>, E>` can obscure exact allocations, even when the iterator length is known.
**Action:** Replace these chains with explicit `Vec::with_capacity(count)` and `for` loops in hot parsing paths (like `duke-classfile/src/parser.rs`) to ensure zero intermediate allocations and explicit sizing.
**[Avoid redundant Vec allocations in read-only equality tests]**
**Learning:** Cloning a full `Vec` just to iterate over it causes O(N) heap allocations, but replacing it with a direct reference `&Vec` requires care to avoid mutability conflicts if the heap is mutated later in the same scope.
**Action:** In read-only scopes (like `native_arrays_equals_int`), replace `.fields.clone()` with `&...fields` and use `.iter().zip()` for zero-cost slice iteration.
