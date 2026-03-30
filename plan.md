1. **Explore `duke-interpreter` for the "Blob" anti-pattern.**
   - The file `crates/duke-interpreter/src/lib.rs` is over 27,000 lines long, which makes it a classic "God Module" and a major structural bottleneck.
2. **Extract the Core Execution Engine.**
   - Create a new module `crates/duke-interpreter/src/engine.rs` to encapsulate the core bytecode interpretation logic (`execute`, `execute_class`, `execute_class_to_completion`, etc.). This isolates the execution state, instruction handling, and frame lifecycle from the massive amount of native method boilerplate present in `lib.rs`.
3. **Refactor and Decouple.**
   - Ensure `lib.rs` uses the new `engine` module via `pub mod engine; pub use engine::*;`.
   - Re-route internal dependencies, constants (`THREAD_TARGET_SLOT`), and reflection helper functions so that `lib.rs` functions appropriately interact with `engine.rs` without circular imports.
4. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
   - Run `cargo check`, `cargo fmt`, `cargo test`, and `cargo clippy`.
5. **Submit the PR.**
