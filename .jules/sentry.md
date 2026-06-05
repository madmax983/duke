## 2026-06-05 - Zip Loader Missing Edge Cases
**Learning:** `find_resource*` family methods in ZipLoader had error propagation logic missing when parsing boot_inf elements. It was missing test coverage. Also format edge cases where the entry exceeds bounds were not tested.
**Action:** Wrote tests and validated correct error handling propagation for nested resources and CD entry bounds.
