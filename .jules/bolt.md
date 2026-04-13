**[JImage Index Map Preallocation]
**Learning:** [HashMap::new() for large, known-size collections causes severe reallocation overhead during startup, especially when parsing files with 30,000+ entries like the JDK jimage file.]
**Action:** [Always use `HashMap::with_capacity(capacity)` when the final size of the collection is already known from headers (e.g., `resource_count`).]
**[Optimize Parser Allocations]
**Learning:** Iterator chains like `.map().collect()` on complex structures returning `Result<Vec<T>, E>` can obscure exact allocations, even when the iterator length is known.
**Action:** Replace these chains with explicit `Vec::with_capacity(count)` and `for` loops in hot parsing paths (like `duke-classfile/src/parser.rs`) to ensure zero intermediate allocations and explicit sizing.
**Eliminate intermediate Vec allocation in StringJoiner**
**Learning:** We identified a hot path string concatenation in `native_stringjoiner_tostring` where it was unnecessarily accumulating string representations into an intermediate `Vec<String>` before joining them.
**Action:** Replaced `.collect::<Vec<_>>()` and `.join()` with a pre-allocated single String buffer and `.push_str()` direct appending. This eliminates overhead for allocating vector buffers and extra formatting strings.
**[Eliminate HashMap Read Heap Allocations]
**Learning:** [Replacing `.clone()` with `&` references for large collection reads (like `HashMap` queries `&heap.get(this_ref)?.fields`) eliminates O(N) heap allocations, but caution is required to ensure it doesn't create overlapping mutable borrows if the lookup function subsequently calls into the VM (e.g. `equals` invoking a method). Here it was safe as the lookup (`slots_equal`) did not mutate.]
**Action:** [Prefer immutable borrows when querying heap structures like HashMaps in the interpreter, verifying first that the inner matching function `slots_equal` doesn't require a mutable reference to the `Heap`.]

**Reduced String.clone() in Registry**
**Learning:** `clone()` operations on large string keys or options inside tight loops (like class registry checks and super_class/interface iterations) unnecessarily allocate memory even when immutable references `&str` or simple value drops are perfectly valid.
**Action:** Replace `clone()` with `.as_deref()` or borrow references (`&ctx.super_class`, `&ctx.interfaces`) when calling external methods, and limit string copies strictly to hash map insertion via moving or single string duplication.

**Basic Block Vec Reallocation**
**Learning:** `Vec::new()` inside looping parser instructions causes multi-level heap reallocations. For small arrays where capacity is known from slice bounds (like basic blocks bounds / leader offsets), `Vec::with_capacity` drastically minimizes heap allocation traffic.
**Action:** When constructing `Vec` from parsed instruction iterators, pre-compute rough capacity based on slice metrics or known leaders array length.
## 2026-04-13 - [Avoid Chained Iterators with Collect]
**Learning:** Replaced `.drain().map().collect::<Vec<_>>().` pattern on a `HashMap` with an explicit `Vec::with_capacity()` and a for-loop. The chained iterators drop the size hint, causing unnecessary heap reallocations. Also, explicit `drop()` on lock guards avoids clippy warnings `clippy::significant_drop_tightening` when used immediately before long-running operations.
**Action:** Use pre-allocated vectors and loops instead of chained iterator `collect`s when the size is known, especially around lock-managed states.
