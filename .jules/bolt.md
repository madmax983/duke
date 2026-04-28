**Optimize Vector Allocations in Streams**
**Learning:** `Vec::new()` requires constant reallocation when pushing elements, which is extremely expensive.
**Action:** When you know the target size of a collection (e.g. mapping over a stream where elements match 1:1), always initialize using `Vec::with_capacity(elems.len())`.

**String Allocation Truncation**
**Learning:** Using `s.chars().take(n).collect::<String>()` allocates an entirely new String under the hood, traversing the utf-8 characters to build it.
**Action:** For string truncation, calculate the precise UTF-8 byte boundary using `.char_indices().nth(n)` and use the in-place `String::truncate(byte_idx)` to perform a zero-allocation length modification.

**String Concatenation with Prefix and Suffix**
**Learning:** Using `filter_map` to build a `Vec<String>`, then `.join()` and finally using `format!` macro for prefix and suffix generates multiple intermediate heap allocations and string operations.
**Action:** Calculate the total capacity, allocate a single `String::with_capacity()`, and `.push_str()` the prefix, items (with delim in between), and suffix directly to eliminate intermediate `Vec`s and `String`s.

**Zero-cost abstractions around `Vec::clone()`**
**Learning:** `heap.get(this_ref)?.fields.clone()` is a heavy operation for HashMaps/HashSets/Localdatetimes operations since we only read from the vector without taking ownership. But you must be careful because passing `&heap.get(this_ref)?.fields` holds a reference to `heap`, and later calling `heap.get_mut` will fail with "cannot borrow `*heap` as mutable because it is also borrowed as immutable".
**Action:** Instead of `fields.clone()`, get the properties out of the reference (`fields[i + 1]`) and use those to perform the operation, or use `find_..._entry_index` to just get the index, then drop the immutable borrow, and then perform `heap.get_mut` if necessary.

**Instruction control_flow_targets allocation trap**
**Learning:** Returning `Vec::with_capacity(2)` instead of `Vec::new()` for CFG targets forces allocations for early returns (0 targets), regressing performance. For typical 1-2 target cases, `Vec::new().push()` and `Vec::with_capacity(2).push()` perform the same initial allocation.
**Action:** Do not use `Vec::with_capacity` if there is a hot early return path that pushes 0 elements, as `Vec::new()` does not allocate until the first push.
