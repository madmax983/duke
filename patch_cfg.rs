<<<<<<< SEARCH
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
#[must_use]
pub fn generate_mermaid_cfg(instructions: &[(usize, Instruction)]) -> String {
=======
/// Generates a Mermaid control flow graph (CFG) from a list of decoded instructions.
///
/// This is used for visualising the structure of a Java method. It outputs
/// Mermaid.js compatible syntax (using `graph TD`). Each instruction becomes a node,
/// and edges represent the control flow between them (e.g. conditional branches, gotos, returns).
///
/// # Examples
///
/// ```
/// use duke_bytecode::{Instruction, generate_mermaid_cfg};
///
/// let instructions = vec![
///     (0, Instruction::Iconst0),
///     (1, Instruction::Ifeq(5)), // jump to PC=6
///     (4, Instruction::Iconst1),
///     (5, Instruction::Ireturn),
///     (6, Instruction::Iconst2),
///     (7, Instruction::Ireturn),
/// ];
/// let cfg = generate_mermaid_cfg(&instructions);
///
/// assert!(cfg.contains("graph TD"));
/// assert!(cfg.contains("node0[\"0: iconst_0\"]"));
/// assert!(cfg.contains("node0 --> node1"));
/// assert!(cfg.contains("node1 -->|true| node6"));
/// ```
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
#[must_use]
pub fn generate_mermaid_cfg(instructions: &[(usize, Instruction)]) -> String {
>>>>>>> REPLACE
