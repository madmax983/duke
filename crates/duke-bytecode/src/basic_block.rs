//! Basic Block Analysis.
//!
//! This module provides functionality to group a flat sequence of decoded
//! JVM instructions into a series of Basic Blocks. A basic block is a straight-line
//! code sequence with no branches in except to the entry and no branches out
//! except at the exit.

use std::collections::BTreeSet;

use crate::Instruction;

/// A basic block of JVM bytecode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    /// The starting program counter (PC) of this basic block.
    pub start_pc: usize,
    /// The program counter (PC) immediately following this basic block.
    pub end_pc: usize,
    /// The decoded instructions contained within this basic block.
    pub instructions: Vec<(usize, Instruction)>,
}

/// Builds a list of basic blocks from a flat sequence of instructions.
///
/// A new basic block starts at:
/// 1. The first instruction.
/// 2. The target of any jump or branch.
/// 3. The instruction immediately following any jump, branch, or return.
///
/// ## Examples
///
/// ```
/// use duke_bytecode::{Instruction, basic_block::{build_basic_blocks, BasicBlock}};
///
/// let instructions = vec![
///     (0, Instruction::Iload1),
///     (1, Instruction::Ifeq(5)), // Branch to PC 6 (1 + 5)
///     (4, Instruction::Iconst1),
///     (5, Instruction::Ireturn),
///     (6, Instruction::Iconst2), // Target of the branch
///     (7, Instruction::Ireturn),
/// ];
///
/// let blocks = build_basic_blocks(&instructions);
/// assert_eq!(blocks.len(), 3);
/// assert_eq!(blocks[0].start_pc, 0); // Contains Iload1, Ifeq
/// assert_eq!(blocks[1].start_pc, 4); // Contains Iconst1, Ireturn
/// assert_eq!(blocks[2].start_pc, 6); // Contains Iconst2, Ireturn
/// ```
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]
#[must_use]
pub fn build_basic_blocks(instructions: &[(usize, Instruction)]) -> Vec<BasicBlock> {
    if instructions.is_empty() {
        return Vec::new();
    }

    let mut leaders = BTreeSet::new();

    // Rule 1: The first instruction is always a leader.
    leaders.insert(instructions[0].0);

    for (i, (pc, instr)) in instructions.iter().enumerate() {
        let is_branch_or_return = match instr {
            Instruction::Ifeq(_)
            | Instruction::Ifne(_)
            | Instruction::Iflt(_)
            | Instruction::Ifge(_)
            | Instruction::Ifgt(_)
            | Instruction::Ifle(_)
            | Instruction::IfIcmpeq(_)
            | Instruction::IfIcmpne(_)
            | Instruction::IfIcmplt(_)
            | Instruction::IfIcmpge(_)
            | Instruction::IfIcmpgt(_)
            | Instruction::IfIcmple(_)
            | Instruction::IfAcmpeq(_)
            | Instruction::IfAcmpne(_)
            | Instruction::Ifnull(_)
            | Instruction::Ifnonnull(_)
            | Instruction::Jsr(_)
            | Instruction::JsrW(_) => {
                // Rule 2: The target of a branch is a leader.
                let offset = match instr {
                    Instruction::JsrW(off) => *off as isize,
                    Instruction::Ifeq(off)
                    | Instruction::Ifne(off)
                    | Instruction::Iflt(off)
                    | Instruction::Ifge(off)
                    | Instruction::Ifgt(off)
                    | Instruction::Ifle(off)
                    | Instruction::IfIcmpeq(off)
                    | Instruction::IfIcmpne(off)
                    | Instruction::IfIcmplt(off)
                    | Instruction::IfIcmpge(off)
                    | Instruction::IfIcmpgt(off)
                    | Instruction::IfIcmple(off)
                    | Instruction::IfAcmpeq(off)
                    | Instruction::IfAcmpne(off)
                    | Instruction::Ifnull(off)
                    | Instruction::Ifnonnull(off)
                    | Instruction::Jsr(off) => isize::from(*off),
                    _ => unreachable!(),
                };
                leaders.insert((*pc as isize + offset) as usize);
                true
            }
            Instruction::Goto(offset) => {
                leaders.insert((*pc as isize + isize::from(*offset)) as usize);
                true
            }
            Instruction::GotoW(offset) => {
                leaders.insert((*pc as isize + *offset as isize) as usize);
                true
            }
            Instruction::Tableswitch {
                default, offsets, ..
            } => {
                leaders.insert((*pc as isize + *default as isize) as usize);
                for offset in offsets {
                    leaders.insert((*pc as isize + *offset as isize) as usize);
                }
                true
            }
            Instruction::Lookupswitch { default, pairs } => {
                leaders.insert((*pc as isize + *default as isize) as usize);
                for (_, offset) in pairs {
                    leaders.insert((*pc as isize + *offset as isize) as usize);
                }
                true
            }
            Instruction::Return
            | Instruction::Ireturn
            | Instruction::Lreturn
            | Instruction::Freturn
            | Instruction::Dreturn
            | Instruction::Areturn
            | Instruction::Athrow
            | Instruction::Ret(_)
            | Instruction::RetW(_) => {
                // Returns and throws also terminate the current block.
                true
            }
            _ => false,
        };

        if is_branch_or_return && i + 1 < instructions.len() {
            // Rule 3: The instruction immediately following a branch/return is a leader.
            leaders.insert(instructions[i + 1].0);
        }
    }

    let mut blocks = Vec::new();
    let mut current_block = Vec::new();
    let mut current_start = instructions[0].0;

    for (pc, instr) in instructions {
        if leaders.contains(pc) && !current_block.is_empty() {
            blocks.push(BasicBlock {
                start_pc: current_start,
                end_pc: *pc,
                instructions: current_block,
            });
            current_block = Vec::new();
            current_start = *pc;
        }
        current_block.push((*pc, instr.clone()));
    }

    if !current_block.is_empty() {
        // We do not have an `encoded_len` method.
        // Let's assume the last instruction takes at least 1 byte.
        let end_pc = current_block
            .last()
            .map_or(current_start + 1, |(pc, _)| *pc + 1);
        blocks.push(BasicBlock {
            start_pc: current_start,
            end_pc,
            instructions: current_block,
        });
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Instruction;

    #[test]
    fn test_build_basic_blocks_empty() {
        let blocks = build_basic_blocks(&[]);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_build_basic_blocks_linear() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Istore1),
            (2, Instruction::Iconst1),
            (3, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].start_pc, 0);
        assert_eq!(blocks[0].instructions.len(), 4);
    }

    #[test]
    fn test_build_basic_blocks_with_branch() {
        let instructions = vec![
            (0, Instruction::Iload1),
            (1, Instruction::Ifeq(5)), // Branch to 6
            (4, Instruction::Iconst1),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst2), // Target
            (7, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].start_pc, 0);
        assert_eq!(blocks[0].instructions.len(), 2); // 0, 1
        assert_eq!(blocks[1].start_pc, 4);
        assert_eq!(blocks[1].instructions.len(), 2); // 4, 5
        assert_eq!(blocks[2].start_pc, 6);
        assert_eq!(blocks[2].instructions.len(), 2); // 6, 7
    }
}
