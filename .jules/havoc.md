## 2024-05-24 - [Classfile Annotation parsing OOM due to deep recursion]
**The Trigger:** A class file containing a deeply nested `RuntimeVisibleAnnotations` attribute (e.g. nested arrays inside annotations inside annotations).
**The Stack Trace:** The process panics with `stack overflow` or crashes due to OOM when allocating vectors at each recursion level, or exceeds system limits.
**Reproduction:** Run `cargo test --test havoc_classfile_annotation_oom` (which we added and fixed).
**Comment:** "You assumed the JVM specification limit of annotation depth was naturally bounded by the class file size, but deep nesting of `[` arrays with 1 element allows exponential stack growth compared to byte size. You were wrong."
## 2025-02-14 - Interrupted Host Threads TOCTOU Race Simulation
**The Weak Point:** Found a TOCTOU race in `duke-interpreter` `native.rs`. The logic sequentially acquires and releases read locks on two synchronized maps (`java_thread_hosts` and `interrupted_host_threads`). Concurrently, `unregister_java_host_thread` can step in between and remove the entries.
**The Wreckage:** Created a `loom` harness that deterministically reproduces the interleaving where a thread gets an ID from the first lock, another thread removes that ID from both maps, and then the first thread checks the second map with the stale ID.
**Action:** Wrote `havoc_interrupted_threads_toctou_loom.rs` to track this race condition in the test suite using Loom. Always verify separated mutex interactions with Loom!
