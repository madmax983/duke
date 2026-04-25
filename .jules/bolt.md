**Optimize Vector Allocations in Streams**
**Learning:** `Vec::new()` requires constant reallocation when pushing elements, which is extremely expensive.
**Action:** When you know the target size of a collection (e.g. mapping over a stream where elements match 1:1), always initialize using `Vec::with_capacity(elems.len())`.

**String Allocation Truncation**
**Learning:** Using `s.chars().take(n).collect::<String>()` allocates an entirely new String under the hood, traversing the utf-8 characters to build it.
**Action:** For string truncation, calculate the precise UTF-8 byte boundary using `.char_indices().nth(n)` and use the in-place `String::truncate(byte_idx)` to perform a zero-allocation length modification.
**Optimize String.join Allocation**
**Learning:** Building strings using a loop and `Vec<String>` allocations (`Vec::with_capacity`, iterating and pushing) along with `format!("{n}")` or `parts.join(&delim)` involves multiple unnecessary temporary allocations.
**Action:** Replace `Vec<String>` buffer allocations in `native_string_join` by appending directly into a pre-allocated `String::new()` instance and writing format arguments without creating intermediate objects using `std::fmt::Write`.
