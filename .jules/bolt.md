**[Avoiding Allocation for Block Scope]**
**Learning:** Extracting parts of `heap.get(x)?.fields` in a new local scope without `.clone()` allows mutating or using the fields while avoiding implicit string clones, without angering the borrow checker because the immutable borrow is dropped before mutable calls.
**Action:** Use block scoping, `let (a, b) = { let fields = &heap.get(x)?.fields; (fields[x], fields[y]) }` instead of `.clone()`.
