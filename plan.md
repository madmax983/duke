1. We successfully extracted the `AtomicPayload` and its associated state structs (`ReentrantLockState`, `ConditionWaiter`, `SemaphoreState`, etc.) from the monolithic `crates/duke-gc/src/lib.rs` into `crates/duke-gc/src/sync.rs`. This correctly splits a "Blob" module.
2. I fixed `duke/src/jar_diff.rs` where it was using `duke_classfile::types` which we must have encapsulated previously but someone missed a reference in `nova` feature path or test path. Wait, I fixed it so now everything compiles.
3. I will make sure the `set_plan` has pre-commit steps.
4. "ATLAS'S JOURNAL - CRITICAL LEARNINGS ONLY... ONLY add journal entries when you discover... A module that has grown too large and needs splitting (The "Blob" anti-pattern)."
   I will append to `.jules/atlas.md`:
```
**[Extract Synchronization Primitives]
**Tangle:** The `duke-gc/src/lib.rs` module had grown into a "Blob" (over 2,500 lines) by mixing core garbage collection logic (like `Heap` and allocation) with JVM concurrency/synchronization primitives (`AtomicPayload`, `ReentrantLockState`, `SemaphoreState`, etc.).
**Blueprint:** Extracted the host-side synchronization structs and `AtomicPayload` into a dedicated `sync.rs` module inside `duke-gc`, restoring high cohesion and reducing module bloat.
```

Wait, `set_plan` needs an explicit format.
Let's formulate the plan.
