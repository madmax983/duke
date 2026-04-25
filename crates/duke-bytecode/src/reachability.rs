//! Control flow reachability analysis for JVM bytecode.
//!
//! This module provides tools to analyze the connectivity of [`BasicBlock`]s.
//! It allows determining execution paths, identifying dead (unreachable) code
//! blocks for dead code elimination, and tracing the shortest path between
//! instructions to aid in verifying structural properties.

#[cfg(feature = "nova")]
use crate::basic_block::BasicBlock;
#[cfg(feature = "nova")]
use std::collections::{HashMap, HashSet, VecDeque};

/// Gets the successor PCs for a given basic block based on its last instruction.
///
/// This is essential for building control flow graphs, as it determines which
/// blocks can be executed immediately after the current one completes.
///
/// # Examples
///
/// ```
/// use duke_bytecode::Instruction;
/// use duke_bytecode::build_basic_blocks;
/// use duke_bytecode::reachability::get_successors;
///
/// let instructions = vec![
///     (0, Instruction::Iconst0),
///     (1, Instruction::Ifeq(5)), // Branch to pc=6 (1 + 5)
///     (4, Instruction::Iconst1),
///     (5, Instruction::Ireturn),
///     (6, Instruction::Iconst2),
///     (7, Instruction::Ireturn),
/// ];
/// let blocks = build_basic_blocks(&instructions);
///
/// // The first block ends with `ifeq 5`, which can branch to pc=6 or fall through to pc=4.
/// let successors = get_successors(&blocks[0]);
/// assert_eq!(successors, vec![6, 4]);
/// ```
#[cfg(feature = "nova")]
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
#[must_use]
pub fn get_successors(block: &BasicBlock) -> Vec<usize> {
    let mut successors = Vec::new();
    if let Some((last_pc, last_instr)) = block.instructions.last() {
        let next_block_id = block.end_pc;
        if last_instr.is_return() {
            // No successors
        } else if let Some(offset) = last_instr.unconditional_jump_target() {
            successors.push((*last_pc as isize + offset) as usize);
        } else if let Some(offset) = last_instr.conditional_branch_target() {
            successors.push((*last_pc as isize + offset) as usize);
            successors.push(next_block_id);
        } else if let Some((default, pairs)) = last_instr.switch_targets() {
            successors.push((*last_pc as isize + default as isize) as usize);
            for (_, offset) in pairs {
                successors.push((*last_pc as isize + offset as isize) as usize);
            }
        } else {
            successors.push(next_block_id);
        }
    }
    successors
}

/// Finds all dead (unreachable) basic blocks starting from the given entry PC.
/// Returns a list of block `start_pc`s that are unreachable.
///
/// Identifying dead blocks is crucial for dead code elimination optimizations
/// and ensuring verifiers don't waste time analyzing unreachable paths.
///
/// # Examples
///
/// ```
/// use duke_bytecode::Instruction;
/// use duke_bytecode::build_basic_blocks;
/// use duke_bytecode::reachability::find_dead_blocks;
///
/// let instructions = vec![
///     (0, Instruction::Goto(6)), // Unconditional jump to pc=6
///     (3, Instruction::Iconst1), // Unreachable dead block
///     (4, Instruction::Ireturn), // Unreachable dead block
///     (6, Instruction::Iconst2), // Target
///     (7, Instruction::Ireturn),
/// ];
/// let blocks = build_basic_blocks(&instructions);
///
/// // Since we jump over pc=3 and pc=4, block starting at 3 is dead.
/// let dead = find_dead_blocks(&blocks, 0);
/// assert_eq!(dead, vec![3]);
/// ```
#[cfg(feature = "nova")]
#[must_use]
pub fn find_dead_blocks(blocks: &[BasicBlock], entry_pc: usize) -> Vec<usize> {
    if blocks.is_empty() {
        return Vec::new();
    }

    let mut block_map = HashMap::new();
    for block in blocks {
        block_map.insert(block.start_pc, block);
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    if block_map.contains_key(&entry_pc) {
        queue.push_back(entry_pc);
        visited.insert(entry_pc);
    }

    while let Some(current_pc) = queue.pop_front() {
        if let Some(block) = block_map.get(&current_pc) {
            let successors = get_successors(block);
            for next_pc in successors {
                if block_map.contains_key(&next_pc) && !visited.contains(&next_pc) {
                    visited.insert(next_pc);
                    queue.push_back(next_pc);
                }
            }
        }
    }

    blocks
        .iter()
        .filter(|b| !visited.contains(&b.start_pc))
        .map(|b| b.start_pc)
        .collect()
}

/// Finds the shortest execution path (in terms of basic block transitions)
/// from the entry block to a target block.
/// Returns a sequence of `start_pc` representing the path, or `None` if unreachable.
///
/// This is helpful for generating trace information or illustrating how
/// a specific state or instruction might be reached during execution.
///
/// # Panics
///
/// Panics if the internal graph traversal fails to find a parent for a node on a confirmed path.
///
/// # Examples
///
/// ```
/// use duke_bytecode::Instruction;
/// use duke_bytecode::build_basic_blocks;
/// use duke_bytecode::reachability::find_shortest_path;
///
/// let instructions = vec![
///     (0, Instruction::Ifeq(8)), // branch to 8
///     (3, Instruction::Goto(7)), // branch to 10
///     (6, Instruction::Ireturn), // unreachable
///     (8, Instruction::Iconst1),
///     (9, Instruction::Ireturn),
///     (10, Instruction::Iconst2),
///     (11, Instruction::Ireturn),
/// ];
/// let blocks = build_basic_blocks(&instructions);
///
/// // Path to pc=10 goes through pc=0, falls through to pc=3, then jumps to pc=10.
/// let path = find_shortest_path(&blocks, 0, 10).unwrap();
/// assert_eq!(path, vec![0, 3, 10]);
/// ```
#[cfg(feature = "nova")]
#[must_use]
pub fn find_shortest_path(
    blocks: &[BasicBlock],
    start_pc: usize,
    target_pc: usize,
) -> Option<Vec<usize>> {
    if blocks.is_empty() {
        return None;
    }

    let mut block_map = HashMap::new();
    for block in blocks {
        block_map.insert(block.start_pc, block);
    }

    if !block_map.contains_key(&start_pc) || !block_map.contains_key(&target_pc) {
        return None;
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut parents: HashMap<usize, usize> = HashMap::new();

    queue.push_back(start_pc);
    visited.insert(start_pc);

    let mut found = false;

    while let Some(current_pc) = queue.pop_front() {
        if current_pc == target_pc {
            found = true;
            break;
        }

        if let Some(block) = block_map.get(&current_pc) {
            for next_pc in get_successors(block) {
                if block_map.contains_key(&next_pc) && !visited.contains(&next_pc) {
                    visited.insert(next_pc);
                    parents.insert(next_pc, current_pc);
                    queue.push_back(next_pc);
                }
            }
        }
    }

    if found {
        let mut path = Vec::new();
        let mut curr = target_pc;
        while curr != start_pc {
            path.push(curr);
            curr = *parents.get(&curr).unwrap();
        }
        path.push(start_pc);
        path.reverse();
        Some(path)
    } else {
        None
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::Instruction;
    use crate::basic_block::build_basic_blocks;

    #[test]
    fn test_get_successors() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Ifeq(5)),
            (4, Instruction::Iconst1),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst2),
            (7, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);

        assert_eq!(blocks.len(), 3);
        let succ0 = get_successors(&blocks[0]);
        assert_eq!(succ0, vec![6, 4]); // 1 + 5 = 6, next = 4
    }

    #[test]
    fn test_find_dead_blocks() {
        let instructions = vec![
            (0, Instruction::Goto(6)),
            (3, Instruction::Iconst1), // Dead block
            (4, Instruction::Ireturn),
            (6, Instruction::Iconst2), // Target
            (7, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);

        let dead = find_dead_blocks(&blocks, 0);
        assert_eq!(dead, vec![3]);
    }

    #[test]
    fn test_find_shortest_path() {
        let instructions = vec![
            (0, Instruction::Ifeq(8)), // branch to 8
            (3, Instruction::Goto(7)), // branch to 10
            (6, Instruction::Ireturn), // unreachable
            (8, Instruction::Iconst1),
            (9, Instruction::Ireturn),
            (10, Instruction::Iconst2),
            (11, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);

        // Path to 10: 0 -> 3 -> 10
        let path = find_shortest_path(&blocks, 0, 10).unwrap();
        assert_eq!(path, vec![0, 3, 10]);

        // Path to 8: 0 -> 8
        let path2 = find_shortest_path(&blocks, 0, 8).unwrap();
        assert_eq!(path2, vec![0, 8]);

        // Unreachable
        assert!(find_shortest_path(&blocks, 0, 6).is_none());
    }
}
