## 2026-05-23 - JDWP ObjectReference OOM
**The Trigger:** A malicious JDWP `ObjectReference.GetValues` payload with an absurdly large length/count parameter (e.g. `0x3FFFFFFF`).
**The Stack Trace:**
```
memory allocation of 10737418240 bytes failed
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
Aborted
```
**Reproduction:** Run `cargo test -p duke --test havoc_jdwp_oom` which starts the JVM in debug mode and fires the bad network packet.
**Comment:** "You trusted the network payload size without question. Now your server is dead."
