**[Title]
**Tangle:** Tried to crash duke_classfile, duke_loader, duke_gc, and duke_interpreter with garbage data and fuzz tests using proptest.
**Blueprint:** Found no panics. The codebase correctly uses `try_from`, `min()`, `checked_add`, `unwrap_or`, and proper Error returning instead of unwrapping blindly on untrusted inputs. I'll summarize these findings and submit.
**[JImage Zip Bomb (OOM)]**
**The Trigger:** A malicious JImage containing highly compressed resources ("zip bombs") could easily exhaust JVM memory during loading because the `DeflateDecoder` allocation logic dynamically responded to embedded header properties without bounding the active stream payload size (`decoder.read_to_end()`).
**Fix Applied:** Introduced a hard cap memory limit (256MB) preventing untrusted `uncompressed_size` capacities from overflowing allocations. Chained `decoder.take(max_size)` explicitly bounding the underlying `flate2::read::DeflateDecoder` to stop unbound inflate output streams.
**Action:** When validating formats that combine lengths inside headers with an inline uncompressed payload block, always sanitize properties but never blindly process unbounded iterators over streams (`read_to_end`, `.collect()`) without `take()`.
