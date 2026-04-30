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

**[SerializeMap for custom map serialization]**
**Learning:** Using `serde::ser::SerializeMap` directly removes the need for collecting intermediate HashMaps when writing a custom map serialization logic.
**Action:** Always prefer iterating directly and using `ser.serialize_map` to prevent unnecessary allocations.

**Zip Entry Iteration Without Intermediate Vectors**
**Learning:** `reader.entry_names().collect::<Vec<String>>()` unnecessarily creates a heap allocation for all string elements and the backing vector, which is very expensive when processing large JAR/ZIP files, especially when you are only iterating over them.
**Action:** When processing `ZipReader` entry names, iterate directly on the `reader.entry_names()` iterator using a `for` loop to prevent unnecessary allocations, ensuring zero-cost abstraction. If `clippy` triggers `case_sensitive_file_extension_comparisons`, use `#[allow(clippy::case_sensitive_file_extension_comparisons)]` inside the `for` loop instead of mapping and collecting.
