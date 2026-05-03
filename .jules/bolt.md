**Remove intermediate allocations in GC mark phase**
**Learning:** Using `.collect::<Vec<_>>()` to create an intermediate vector just to immediately feed it into `.extend(...)` on a worklist in a hot loop (like a GC mark phase) introduces unnecessary heap allocations per object.
**Action:** Chain the iterator directly into `.extend(...)` to consume it immediately, satisfying the borrow checker and providing a zero-cost abstraction that completely eliminates the temporary vector allocation.
