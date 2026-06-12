## 2025-02-12 - Telemetry OOM DOS fix
**The Trigger:** Calling native methods with many distinct, unique string combinations resulting in an unbounded accumulation of strings in the `NativeBoundaryStore` stats map.
**The Stack Trace:** Panic due to OOM when the memory allocation exceeded the limits.
**Reproduction:** A simple loop generating `1_000_000` unique classes in a test triggered OOM easily.
**Comment:** Memory structures that grow dynamically based on untrusted or external input MUST have explicit capacity bounds (or an LRU cache) to prevent resource exhaustion attacks.
