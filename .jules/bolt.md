**Avoid Deep Clones in GC Phase**
**Learning:** During a minor GC phase, moving an object using `.clone()` introduces massive allocations for nested elements (like `fields` vecs and `string`s). Moving the object via `.take()` and leaving a dummy forwarding object is zero-cost and preserves correctness.
**Action:** Instead of `let copy = obj.clone()`, use `let copy = self.young[y_idx].take().unwrap()` to move the data, followed by installing a `forward: Some(new_ref)` tombstone in its place.
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
**[Eliminate HashMap/ArrayList `.fields.clone()` Allocations in Native Bindings]**
**Learning:** [Many Java collection native mappings (like `ArrayList.remove/contains`, `HashMap.keySet/values`, `LocalDateTime.isAfter`) were cloning the entire `fields` vector from `duke_gc::Heap` objects just to iterate or read elements. Because the heap lookup functions (`heap.get`) return a reference to the `HeapObject`, we can just borrow `.fields` immutably for loops, or do direct length/index lookups, completely avoiding massive `Vec` cloning and reallocation overhead on hot execution paths.]
**Action:** [Use immutable references `&heap.get(...)?.fields` for native execution functions that only need to read internal Java object states or iterate them. If mutation is required after a lookup, ensure the immutable borrow is dropped (e.g. by wrapping the lookup loop in a `{ ... }` block) before calling `heap.get_mut()`.]
## 2026-04-13 - [Avoid Chained Iterators with Collect]
**Learning:** Replaced `.drain().map().collect::<Vec<_>>().` pattern on a `HashMap` with an explicit `Vec::with_capacity()` and a for-loop. The chained iterators drop the size hint, causing unnecessary heap reallocations. Also, explicit `drop()` on lock guards avoids clippy warnings `clippy::significant_drop_tightening` when used immediately before long-running operations.
**Action:** Use pre-allocated vectors and loops instead of chained iterator `collect`s when the size is known, especially around lock-managed states.
**[Eliminate Slot Vec clones in Native Array/HashMap Iteration]
**Learning:** [Many Java native methods like `native_arraylist_index_of` or `native_arrays_equals_int` were cloning the entire `fields` vector from `heap.get(this_ref)` simply to iterate and search. Since `Slot` is `Copy`, cloning the entire vector is wildly inefficient. A previous learning suggested borrowing with `&heap.get(...)?.fields`, but this causes borrow checker conflicts if the inner loop needs `heap` for operations like `slots_equal`.]
**Action:** [Use an index-based loop (`for i in 0..len`) by getting `len` first, then retrieving `heap.get(this_ref)?.fields[i]` inside the loop. This avoids both full `Vec` cloning and holding overlapping `Heap` borrows across function calls.]
**[Optimizing Collection Construction in Loops]
**Learning:** Iteratively pushing elements and cloning inside a loop into a `Vec` is measurably slower than taking a slice and calling `to_vec()` or `extend_from_slice()`. The slice operations can pre-allocate the exact required capacity and use more efficient batch operations instead of loop-driven reallocations and individual `.clone()` calls.
**Action:** When gathering items from an existing slice/vector into sub-vectors (like segmenting basic blocks), keep track of slice indices instead of accumulating into a temporary `Vec` element by element. Convert the finalized slice via `to_vec()` when the boundary is reached.
**[String Substring Allocations]
**Learning:** `.chars().collect::<String>()` allocates a temporary vector of characters under the hood before creating the new string.
**Action:** Use `char_indices().nth(index)` to find precise byte bounds, validate against `chars().count()` for JVM UTF-16 compatibility, and use native Rust `&str[start..end]` slicing to prevent intermediate allocation and significantly boost performance.
