1. **Refactor "The Leak" in `duke-bytecode`, `duke-classfile`, `duke-telemetry`, and `duke-interpreter`**
   - The crates use a facade pattern but leak internal implementation details via wildcard exports like `pub use ...::*` and `pub use crate::...::*`. I will replace these with explicit item exports to form a strict API boundary.
   - Use `run_in_bash_session` with a Python script and `re.sub` to update:
     - `crates/duke-classfile/src/lib.rs` (replace wildcard exports in `pub mod types` and explicitly export AttributeData, ClassFile, etc.)
     - `crates/duke-interpreter/src/lib.rs` (replace `pub use context::*;`, `pub use registry::*;` with explicit types)
     - `crates/duke-telemetry/src/lib.rs` (replace wildcard exports like `pub use bytecode_cost::*;` with explicitly naming structs)

2. **Verify the structural changes**
   - Use `run_in_bash_session` to run `cargo test`, `cargo check`, and `cargo clippy --all-targets --all-features -- -D warnings`.
   - Also, use `git diff` to review the modifications made in step 1.

3. **Complete pre-commit steps**
   - Call the `pre_commit_instructions` tool and follow the steps to ensure proper testing, verification, review, and reflection are done.

4. **Commit the changes**
   - Execute `git checkout -b atlas-explicit-exports`, `git add -u`, and `git commit -m "🗺️ Atlas: [Explicit API boundaries]"` using `run_in_bash_session`.
