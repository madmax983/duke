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
**Pre-allocate HashMap with `HashMap::with_capacity` when size is known**
**Learning:** `HashMap::new()` requires constant reallocation when inserting elements, which is extremely expensive, especially in critical paths like parsing large class files or JAR headers where the size is easily knowable beforehand.
**Action:** Always prefer `HashMap::with_capacity` instead of `HashMap::new` when the capacity of the map is known up front to eliminate unnecessary intermediate memory allocations and resizing.
**Pre-allocate String capacity instead of format! macro**
**Learning:** Using `format!("prefix{value}")` inside loops or hot paths creates unnecessary intermediate heap allocations and string operations that slow down the application.
**Action:** When creating strings from known prefixes/suffixes, calculate the exact length with `String::with_capacity(prefix.len() + value.len())` and use `push_str()` directly to achieve zero-cost abstraction.

**Avoid Unnecessary Vector Allocations During Iterator Processing**
**Learning:** Functions that process iterators and convert elements into collections (like decoding UTF-16 units into a `String`) often don't need intermediate vectors. In `decode_utf16_bytes`, an entire `Vec<u16>` was pre-allocated and populated just to be immediately consumed by `char::decode_utf16`.
**Action:** When a function accepts an iterator or a slice, design internal helpers to accept `impl IntoIterator` rather than concrete `Vec`s. Here, modifying `decode_utf16_bytes` to pass a mapped `.chunks_exact` iterator directly to `char::decode_utf16` eliminated a full intermediate O(N) heap allocation, adhering to the "Zero-cost abstractions are the law. Memory allocations are the enemy" philosophy.
**[IO Bottleneck in Native Methods]**
**Learning:** `native_file_input_stream_read_bytes` and `native_properties_load` were manually reading byte-by-byte in a tight loop via `read_host_file_byte`. Each iteration crossed the host file boundary, incurring dynamic dispatch overhead, `unwrap` checks, and a potential native syscall, severely bottlenecking data parsing.
**Action:** Implemented a chunked read method `read_host_file_bytes` on `duke_gc::Heap` to efficiently read large blocks natively in one sys-read (or buffered read) operation.
**Trust the Iterator: .collect() is Optimized**
**Learning:** In Rust, `.collect::<Vec<_>>()` called on an `ExactSizeIterator` (which `slice.iter().map()` implements) already knows the exact number of elements. The standard library heavily optimizes this via the `TrustedLen` trait to allocate the precise capacity upfront and completely bypass bounds checks during insertion.
**Action:** Do not manually replace `.collect::<Vec<_>>()` with `Vec::with_capacity` and a `for` loop, as it re-introduces bounds checks on every push, making the "optimization" unidiomatic and a micro-regression.
**Optimize Collections.swap with zero-cost slice::swap**
**Learning:** Swapping elements in  objects by cloning the entire  vector is extremely expensive (O(N) heap allocation) just to mutate two indices.
**Action:** Use  to perform an in-place swap, eliminating the allocation and multiple mutable gets.
**Optimize Collections.swap with zero-cost slice::swap**
**Learning:** Swapping elements in `duke_gc::Heap` objects by cloning the entire `fields` vector is extremely expensive (O(N) heap allocation) just to mutate two indices.
**Action:** Use `heap.get_mut(list_ref)?.fields.swap(fi, fj)` to perform an in-place swap, eliminating the allocation and multiple mutable gets.
