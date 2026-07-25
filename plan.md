1. **Explore & Ideate (The Spark)**
   - We have `duke-bytecode` which provides structural analysis and instruction decoding.
   - We have `similarity.rs` in `duke-bytecode` which implements `calculate_similarity` to compute bytecode sequence similarity using Levenshtein distance on instruction variants (ignoring constants/locals).
   - Currently, there's no CLI tool that utilizes this to find *clones* (copy-pasted or highly similar code) across an entire `.jar`.
   - **Idea: The Plagiarism Detector / Clone Hunter**. Add a new command `duke clone-hunt <file.jar> [similarity_threshold]`. It will analyze all methods in a JAR, compare them against each other using `calculate_similarity`, and report methods that are highly similar (e.g., similarity > 0.95).

2. **Prototype (The Scaffold)**
   - Create `duke/src/clone_hunt.rs` behind `#[cfg(feature = "nova")]`.
   - Iterate all classes and methods in the JAR.
   - Extract their bytecode and decode it to `Vec<Instruction>`.
   - Store them as `(class_name, method_name, Vec<Instruction>)`.
   - Compare every pair of methods (nested loop or itertools combinations) where the instruction sequence length > some minimum (e.g., > 10 instructions to ignore trivial getters/setters).
   - If `calculate_similarity` > threshold (default 0.95), print the match.

3. **Unslop (The Sanity Check)**
   - Add it to `duke/src/main.rs` as a new command.
   - Add a test in `clone_hunt.rs` to ensure it compiles and has basic test coverage.
   - Run `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cargo fmt --all`.

4. **Review & Pre-Commit**
   - Review plan, execute, and verify against boundaries.
