**JImage Location Attributes Bounds Check**
**Learning:** Slices bounds checking does not magically map to logical data structure bounds when multiple structures are packed consecutively into a single array slice.
**Action:** When extracting data based on embedded offsets/lengths inside a multi-region buffer (like a jimage index where `locs` and `strings` are sequential), always bounds-check against the logical end of the *specific region* being parsed (`locs_end`), not just the end of the entire buffer (`data.len()`).
