**[Heap Allocation & Overlapping Mutability]
**Learning:** When replacing `.collect::<Vec<_>>()` chains with a zero-cost abstraction over a GC heap (like `allocate_from_iter(heap, iter)`), you cannot pass the heap mutably to the function while also borrowing it mutably inside the iterator's `map` closure to allocate individual elements.
**Action:** Pre-allocate the target array on the heap empty (e.g., filled with null references), then use a standard `for` loop to sequentially allocate the elements and update the array slots in-place.
