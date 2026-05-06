**[Title]
**Tangle:** Tried to crash duke_classfile, duke_loader, duke_gc, and duke_interpreter with garbage data and fuzz tests using proptest.
**Blueprint:** Found no panics. The codebase correctly uses `try_from`, `min()`, `checked_add`, `unwrap_or`, and proper Error returning instead of unwrapping blindly on untrusted inputs. I'll summarize these findings and submit.
## 2024-05-06 - Zip & JImage DOS / OOM Boundaries
**Learning:** `JImageReader` and `ZipReader` lack sufficient limits on uncompressed buffers or capacity allocations when encountering arbitrarily huge values from potentially untrusted inputs. This can lead to OOM or capacity-overflow panics. Moreover, TOCTOU vulnerabilities and simple locking deadlocks were absent from test coverage for critical VM resources like the property map and threads map.
**Action:** Always add fuzz tests simulating enormous capacity parameters (especially `Vec::with_capacity` patterns) to bounds-check untrusted metadata sizes against hard caps (e.g., 256MB). Use `loom` tests for every `Arc<Mutex>` or `Arc<RwLock>` that performs conditional fetches/modifications to prove atomicity boundaries exist.
