1. **Refactor `run_execution` instruction logic into dedicated modules/functions:**
   The `run_execution` function in `crates/duke-interpreter/src/execution.rs` is over 3,000 lines long, making it a "God Function". I will extract logic for instruction groups into dedicated helper functions.

   To avoid refactoring complexities with control flow macros (`continue`, `return`, `throw_java!`), I will extract only the instructions that purely mutate the stack, local variables, or heap. The new functions will take `&Instruction`, `&mut Frame` (and `&mut Heap` if needed), and return `Result<()>`. The matching will delegate to these functions in single lines.

   **Categories to extract:**
   - `execute_loads_stores(instr: &Instruction, frame: &mut Frame) -> Result<()>`
   - `execute_math(instr: &Instruction, frame: &mut Frame) -> Result<()>`
   - `execute_stack(instr: &Instruction, frame: &mut Frame) -> Result<()>`
   - `execute_conversions(instr: &Instruction, frame: &mut Frame) -> Result<()>`

   Each extracted function will consist of a small `match instr { ... }` that handles its subset of opcodes and returns `Ok(())` or an error if the opcode isn't handled.

   The main `match &instr` block in `run_execution` will be updated to collapse dozens of arms into grouped arms:
   ```rust
   Instruction::Iadd | Instruction::Ladd | ... => execute_math(instr, frame)?,
   ```

2. **Verify changes successfully compiled and test passing:**
   I will run `cargo build` and verify that the match logic correctly handles all variants.
   I will run `cargo test --workspace` to ensure that this purely structural refactoring doesn't break any runtime behaviors.

3. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
