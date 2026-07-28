//! Static Bytecode Smells Analyzer.
//!
//! This module provides a basic static analyzer to find common "code smells"
//! or anti-patterns in JVM bytecode.

#[cfg(feature = "nova")]
use crate::Instruction;

/// Represents a detected code smell in the bytecode.
#[cfg(feature = "nova")]
#[derive(Debug, PartialEq, Eq)]
pub enum BytecodeSmell {
    /// An instruction jumps directly to itself, creating a trivial infinite loop.
    InfiniteLoop {
        /// The program counter of the jump instruction.
        pc: usize,
    },
    /// A value is pushed onto the stack and immediately popped.
    PushImmediatePop {
        /// The program counter of the push instruction.
        pc: usize,
    },
    /// An unconditional jump targets the immediate next instruction, which is redundant.
    RedundantGoto {
        /// The program counter of the redundant jump.
        pc: usize,
    },
}

/// Analyzes a sequence of decoded instructions for common bytecode smells.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "nova")] {
/// use duke_bytecode::Instruction;
/// use duke_bytecode::{BytecodeSmell, analyze_smells};
///
/// let instructions = vec![
///     (0, Instruction::Iconst0),
///     (1, Instruction::Pop),
///     (2, Instruction::Goto(0)),
/// ];
///
/// let smells = analyze_smells(&instructions);
/// assert!(smells.contains(&BytecodeSmell::PushImmediatePop { pc: 0 }));
/// assert!(smells.contains(&BytecodeSmell::InfiniteLoop { pc: 2 }));
/// # }
/// ```
#[cfg(feature = "nova")]
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
#[must_use]
pub fn analyze_smells(instructions: &[(usize, Instruction)]) -> Vec<BytecodeSmell> {
    let mut smells = Vec::new();

    for (i, (pc, instr)) in instructions.iter().enumerate() {
        let next_pc = instructions.get(i + 1).map(|(p, _)| *p);

        match instr {
            Instruction::Goto(offset) => {
                if *offset == 0 {
                    smells.push(BytecodeSmell::InfiniteLoop { pc: *pc });
                } else {
                    #[allow(clippy::collapsible_if)]
                    if let Some(npc) = next_pc {
                        if (*pc as isize + *offset as isize) as usize == npc {
                            smells.push(BytecodeSmell::RedundantGoto { pc: *pc });
                        }
                    }
                }
            }
            Instruction::GotoW(offset) => {
                if *offset == 0 {
                    smells.push(BytecodeSmell::InfiniteLoop { pc: *pc });
                } else {
                    #[allow(clippy::collapsible_if)]
                    if let Some(npc) = next_pc {
                        if (*pc as isize + *offset as isize) as usize == npc {
                            smells.push(BytecodeSmell::RedundantGoto { pc: *pc });
                        }
                    }
                }
            }
            Instruction::Iconst0
            | Instruction::Iconst1
            | Instruction::Iconst2
            | Instruction::Iconst3
            | Instruction::Iconst4
            | Instruction::Iconst5
            | Instruction::IconstM1
            | Instruction::AconstNull
            | Instruction::Fconst0
            | Instruction::Fconst1
            | Instruction::Fconst2
            | Instruction::Dconst0
            | Instruction::Dconst1
            | Instruction::Lconst0
            | Instruction::Lconst1
            | Instruction::Bipush(_)
            | Instruction::Sipush(_) => {
                if let Some((_, Instruction::Pop)) = instructions.get(i + 1) {
                    smells.push(BytecodeSmell::PushImmediatePop { pc: *pc });
                }
            }
            _ => {}
        }
    }

    smells
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::Instruction;

    #[test]
    fn test_analyze_smells_empty() {
        let smells = analyze_smells(&[]);
        assert!(smells.is_empty());
    }

    #[test]
    fn test_infinite_loop_goto() {
        let instructions = vec![(0, Instruction::Goto(0))];
        let smells = analyze_smells(&instructions);
        assert_eq!(smells, vec![BytecodeSmell::InfiniteLoop { pc: 0 }]);
    }

    #[test]
    fn test_infinite_loop_goto_w() {
        let instructions = vec![(0, Instruction::GotoW(0))];
        let smells = analyze_smells(&instructions);
        assert_eq!(smells, vec![BytecodeSmell::InfiniteLoop { pc: 0 }]);
    }

    #[test]
    fn test_redundant_goto() {
        let instructions = vec![(0, Instruction::Goto(3)), (3, Instruction::Ireturn)];
        let smells = analyze_smells(&instructions);
        assert_eq!(smells, vec![BytecodeSmell::RedundantGoto { pc: 0 }]);
    }

    #[test]
    fn test_redundant_goto_w() {
        let instructions = vec![(0, Instruction::GotoW(3)), (3, Instruction::Ireturn)];
        let smells = analyze_smells(&instructions);
        assert_eq!(smells, vec![BytecodeSmell::RedundantGoto { pc: 0 }]);
    }

    #[test]
    fn test_push_immediate_pop() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Pop),
            (2, Instruction::Bipush(42)),
            (4, Instruction::Pop),
        ];
        let smells = analyze_smells(&instructions);
        assert_eq!(
            smells,
            vec![
                BytecodeSmell::PushImmediatePop { pc: 0 },
                BytecodeSmell::PushImmediatePop { pc: 2 },
            ]
        );
    }
}
