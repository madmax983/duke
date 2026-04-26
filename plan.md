1. **Optimize `JoiningCollector` joining in `duke-interpreter/src/native.rs`**
   - The original code used a filter-map to build a `Vec<String>`, then `.join(&delim)` inside `format!("{prefix}{joined}{suffix}")`.
   - This caused multiple intermediate string allocations.
   - We replaced it with a single `String::with_capacity` allocation that iterates through `elems` and pushes strings directly, avoiding `.join()` and `format!`.
   - The test pass successfully and benchmarking shows improvement or at least similar performance with less allocation.
   - We will append our learning to `.jules/bolt.md`.
2. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
3. **Submit the change**
   - Submit the PR with the required Bolt format.
