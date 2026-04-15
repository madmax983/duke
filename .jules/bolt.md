**Avoid Deep Clones in GC Phase**
**Learning:** During a minor GC phase, moving an object using `.clone()` introduces massive allocations for nested elements (like `fields` vecs and `string`s). Moving the object via `.take()` and leaving a dummy forwarding object is zero-cost and preserves correctness.
**Action:** Instead of `let copy = obj.clone()`, use `let copy = self.young[y_idx].take().unwrap()` to move the data, followed by installing a `forward: Some(new_ref)` tombstone in its place.
