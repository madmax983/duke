**[JImage Index Map Preallocation]
**Learning:** [HashMap::new() for large, known-size collections causes severe reallocation overhead during startup, especially when parsing files with 30,000+ entries like the JDK jimage file.]
**Action:** [Always use `HashMap::with_capacity(capacity)` when the final size of the collection is already known from headers (e.g., `resource_count`).]
**[Optimize Parser Allocations]
**Learning:** Iterator chains like `.map().collect()` on complex structures returning `Result<Vec<T>, E>` can obscure exact allocations, even when the iterator length is known.
**Action:** Replace these chains with explicit `Vec::with_capacity(count)` and `for` loops in hot parsing paths (like `duke-classfile/src/parser.rs`) to ensure zero intermediate allocations and explicit sizing.
**Eliminate intermediate Vec allocation in StringJoiner**
**Learning:** We identified a hot path string concatenation in `native_stringjoiner_tostring` where it was unnecessarily accumulating string representations into an intermediate `Vec<String>` before joining them.
**Action:** Replaced `.collect::<Vec<_>>()` and `.join()` with a pre-allocated single String buffer and `.push_str()` direct appending. This eliminates overhead for allocating vector buffers and extra formatting strings.
