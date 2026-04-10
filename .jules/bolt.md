**[JImage Index Map Preallocation]
**Learning:** [HashMap::new() for large, known-size collections causes severe reallocation overhead during startup, especially when parsing files with 30,000+ entries like the JDK jimage file.]
**Action:** [Always use `HashMap::with_capacity(capacity)` when the final size of the collection is already known from headers (e.g., `resource_count`).]
**[Optimize Parser Allocations]
**Learning:** Iterator chains like `.map().collect()` on complex structures returning `Result<Vec<T>, E>` can obscure exact allocations, even when the iterator length is known.
**Action:** Replace these chains with explicit `Vec::with_capacity(count)` and `for` loops in hot parsing paths (like `duke-classfile/src/parser.rs`) to ensure zero intermediate allocations and explicit sizing.
**[Avoid recursive total_instance_field_count]**
**Learning:** When calculating field slot indices using `field_slot_idx`, traversing up the class hierarchy and summing instance fields recursively leads to an O(N^2) complexity because `total_instance_field_count` also iterates through the superclasses. This causes performance overhead for deep hierarchies.
**Action:** Inline the logic of `total_instance_field_count` directly inside the `field_slot_idx` upward traversal loop to maintain O(N) complexity for field resolution.
