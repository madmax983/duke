## 2024-05-24 - Pre-allocating HashMap
**Learning:** Found multiple instances where a `HashMap` was being built from an iterator using `.collect()` on a hot path, causing intermediate reallocation overhead. Since the exact number of elements (`instructions.len()`) was known, this was an unnecessary cost.
**Action:** Replace `.collect()` with manual loop and `HashMap::with_capacity(len)` to avoid multiple reallocations on hot paths.

## 2025-02-23 - Pre-allocating HashMap vs Iterator .collect()
**Learning:** Found multiple instances where a `HashMap` was being built from an iterator using `.collect()` on a hot path. The code review pointed out that in Rust, calling `.collect()` on an `ExactSizeIterator` (which `slice.iter().enumerate().map()` is) already utilizes the iterator's `size_hint()` to pre-allocate the exact capacity required. Replacing the idiomatic `.collect()` with `HashMap::with_capacity()` and a manual `for` loop provides zero performance benefit and makes the code less idiomatic.
**Action:** Do not manually unroll `.collect()` for exact size iterators. Leave it as is because `.collect()` is highly optimized under the hood and already handles the exact capacity.
