//! Basic Block Analysis.
//!
//! This module provides functionality to group a flat sequence of decoded
//! JVM instructions into a series of Basic Blocks. A basic block is a straight-line
//! code sequence with no branches in except to the entry and no branches out
//! except at the exit.

use crate::Instruction;

/// A basic block of JVM bytecode.
///
/// # Examples
///
/// ```
/// use duke_bytecode::{Instruction, BasicBlock};
///
/// let block = BasicBlock {
///     start_pc: 0,
///     end_pc: 2,
///     instructions: vec![
///         (0, Instruction::Iconst1),
///         (1, Instruction::Istore1),
///     ],
/// };
///
/// assert_eq!(block.instructions.len(), 2);
/// ```
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
/// use duke_bytecode::{Instruction, build_basic_blocks, BasicBlock};
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
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
#[must_use]
pub fn build_basic_blocks(instructions: &[(usize, Instruction)]) -> Vec<BasicBlock> {
    if instructions.is_empty() {
        return Vec::new();
    }

    let leaders = find_leaders(instructions);
    construct_blocks(instructions, &leaders)
}

#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
fn find_leaders(instructions: &[(usize, Instruction)]) -> Vec<usize> {
    let mut leaders = Vec::with_capacity(instructions.len() / 4 + 2);
    leaders.push(instructions[0].0);

    for (i, (pc, instr)) in instructions.iter().enumerate() {
        let is_branch_or_return = instr.is_conditional_branch()
            || instr.is_unconditional_jump()
            || instr.is_switch()
            || instr.is_return();

        // Target of any jump or branch is a leader
        for target in instr.control_flow_targets(*pc, None) {
            leaders.push(target);
        }

        // The instruction immediately following any jump, branch, or return is a leader
        if is_branch_or_return && i + 1 < instructions.len() {
            leaders.push(instructions[i + 1].0);
        }
    }
    leaders.sort_unstable();
    leaders.dedup();
    leaders
}

fn construct_blocks(instructions: &[(usize, Instruction)], leaders: &[usize]) -> Vec<BasicBlock> {
    let mut blocks = Vec::with_capacity(leaders.len());
    let mut current_start_pc = instructions[0].0;
    let mut current_start_idx = 0;

    for (i, (pc, _)) in instructions.iter().enumerate() {
        if leaders.binary_search(pc).is_ok() && i > current_start_idx {
            blocks.push(BasicBlock {
                start_pc: current_start_pc,
                end_pc: *pc,
                instructions: instructions[current_start_idx..i].to_vec(),
            });
            current_start_idx = i;
            current_start_pc = *pc;
        }
    }

    if current_start_idx < instructions.len() {
        let current_slice = &instructions[current_start_idx..];
        let end_pc = current_slice
            .last()
            .map_or(current_start_pc + 1, |(pc, _)| *pc + 1);
        blocks.push(BasicBlock {
            start_pc: current_start_pc,
            end_pc,
            instructions: current_slice.to_vec(),
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
