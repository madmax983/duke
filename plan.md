1.  **Refactor `find_leaders` in `crates/duke-bytecode/src/basic_block.rs` to avoid `BTreeSet`**:
    - Change the return type of `find_leaders` from `BTreeSet<usize>` to `Vec<usize>`.
    - Change `leaders.insert(x)` to `leaders.push(x)`.
    - At the end of `find_leaders`, sort and deduplicate the vector: `leaders.sort_unstable(); leaders.dedup();`.
    - Update `construct_blocks` to accept `&[usize]` instead of `&BTreeSet<usize>`.
    - Replace `leaders.contains(pc)` with `leaders.binary_search(pc).is_ok()` in `construct_blocks`.
    - Add a doc comment explaining the optimization.

2.  **Fix unresolved import error**:
    - `jar_diff.rs` was breaking compilation due to an issue with `duke_classfile::types` missing. Refactor the `jar_diff.rs` imports and `and_then` closure type annotations to resolve compilation errors resulting from changes made during previous optimizations.

3.  **Complete pre-commit steps**:
    - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

4.  **Submit the change**:
    - Submit the PR with the title '⚡ Bolt: Replace BTreeSet with Vec in build_basic_blocks'.
