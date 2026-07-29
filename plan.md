1. **Refactor `decode_one` in `crates/duke-bytecode/src/decoder.rs`**:
   - Extract the `op::POP..=op::SWAP` block inside `decode_one` into a new helper function called `decode_stack_op(opcode: u8) -> Result<Instruction>`.
   - Update `decode_one` to call `decode_stack_op(opcode)`.
   - Ensure the new function is placed *above* `decode_one` (so it appears before outer attributes) as per Forge's constraints and memory.

2. **Review Journal & Guidelines**:
   - Check if `decode_one` can be completely flattened, as the instructions state: "When refactoring or adding opcodes, avoid inlining logic in the main `decode_one` match block; extract it into dedicated helper functions to maintain a flat structure."

3. **Verify the change**:
   - Run `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test`.
   - The test should pass successfully and there should be no warnings.

4. **Add journal entry to `.jules/forge.md`**:
   - Record the extraction of `decode_stack_op` into a helper function to avoid inlined match logic inside `decode_one`.

5. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done**.

6. **Request code review**:
   - Run `git add` and request code review to finalize the PR.
