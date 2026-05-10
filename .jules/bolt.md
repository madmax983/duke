**[BTreeSet vs Vec in Hot Paths]**
**Learning:** `BTreeSet` causes significant O(N) allocation overhead per element inserted. When collecting a small set of primitive keys (like `usize` PCs) for later lookup, using a pre-allocated `Vec` followed by `.sort_unstable()`, `.dedup()`, and `.binary_search()` is vastly faster and reduces heap pressure during CFG generation.
**Action:** Replace `BTreeSet` or `HashSet` with sorted `Vec` + `binary_search()` when building single-use collections in performance-critical loops.
