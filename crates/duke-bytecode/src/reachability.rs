//! Basic block reachability and pathfinding.
//!
//! This module provides algorithms for analyzing control flow reachability
//! within a method's basic blocks. It includes utilities to determine basic
//! block successors, identify dead (unreachable) blocks, and find the shortest
//! execution paths between blocks.

#[cfg(feature = "nova")]
use crate::basic_block::BasicBlock;
#[cfg(feature = "nova")]
use std::collections::VecDeque;

/// Gets the successor program counters (PCs) for a given [`BasicBlock`] based on its last instruction.
///
/// This function examines the final instruction of the block (e.g., a branch, a return, or a regular instruction)
/// to determine which instructions could potentially be executed next. This is fundamental for constructing
/// a complete control flow graph.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "nova")] {
/// use duke_bytecode::Instruction;
/// use duke_bytecode::{BasicBlock, build_basic_blocks};
/// use duke_bytecode::get_successors;
///
/// // An unconditional jump to PC 6
/// let instructions = vec![(0, Instruction::Goto(6)), (3, Instruction::Ireturn), (6, Instruction::Ireturn)];
/// let blocks = build_basic_blocks(&instructions);
///
/// let successors = get_successors(&blocks[0]);
/// assert_eq!(successors, vec![6]);
/// # }
/// ```
#[cfg(feature = "nova")]
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
#[must_use]
/// Optimization: Pre-allocate capacity of 2 since branch targets and fallthroughs max out at two.
/// Reduces dynamic reallocation overhead during CFG construction.
pub fn get_successors(block: &BasicBlock) -> Vec<usize> {
    if let Some((last_pc, last_instr)) = block.instructions.last() {
        last_instr.control_flow_targets(*last_pc, Some(block.end_pc))
    } else {
        Vec::new()
    }
}

/// Finds all dead (unreachable) basic blocks starting from the given entry PC.
///
/// A basic block is considered "dead" if there is no valid control flow path from the entry
/// point to that block. Dead code can occur from unoptimized compilation, such as blocks
/// after an unconditional `return` or `goto` that lack any jump targets pointing to them.
///
/// Returns a list of block `start_pc`s that are unreachable.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "nova")] {
/// use duke_bytecode::Instruction;
/// use duke_bytecode::build_basic_blocks;
/// use duke_bytecode::find_dead_blocks;
///
/// let instructions = vec![
///     (0, Instruction::Goto(6)),
///     (3, Instruction::Iconst1), // Dead block! We jump right over it.
///     (4, Instruction::Ireturn),
///     (6, Instruction::Iconst2), // Target of the goto
///     (7, Instruction::Ireturn),
/// ];
/// let blocks = build_basic_blocks(&instructions);
///
/// // Start analysis from entry PC 0
/// let dead = find_dead_blocks(&blocks, 0);
/// assert_eq!(dead, vec![3]);
/// # }
/// ```
#[cfg(feature = "nova")]
#[must_use]
pub fn find_dead_blocks(blocks: &[BasicBlock], entry_pc: usize) -> Vec<usize> {
    if blocks.is_empty() {
        return Vec::new();
    }

    let Ok(entry_idx) = blocks.binary_search_by_key(&entry_pc, |b| b.start_pc) else {
        return blocks.iter().map(|b| b.start_pc).collect();
    };

    let mut visited = vec![false; blocks.len()];
    let mut queue = VecDeque::with_capacity(blocks.len());

    queue.push_back(entry_idx);
    visited[entry_idx] = true;

    while let Some(current_idx) = queue.pop_front() {
        let block = &blocks[current_idx];
        let successors = get_successors(block);
        for next_pc in successors {
            if let Ok(next_idx) = blocks.binary_search_by_key(&next_pc, |b| b.start_pc)
                && !visited[next_idx]
            {
                visited[next_idx] = true;
                queue.push_back(next_idx);
            }
        }
    }

    blocks
        .iter()
        .enumerate()
        .filter(|(idx, _)| !visited[*idx])
        .map(|(_, b)| b.start_pc)
        .collect()
}

/// Finds the shortest execution path (in terms of basic block transitions)
/// from the entry block to a target block.
///
/// This uses Breadth-First Search (BFS) to traverse the control flow graph.
/// It is useful for test generation, coverage analysis, and finding the
/// quickest way to reach a specific block of bytecode.
///
/// Returns a sequence of `start_pc`s representing the path, or `None` if unreachable.
///
/// # Panics
///
/// Panics if the internal graph traversal fails to find a parent for a node on a confirmed path.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "nova")] {
/// use duke_bytecode::Instruction;
/// use duke_bytecode::build_basic_blocks;
/// use duke_bytecode::find_shortest_path;
///
/// let instructions = vec![
///     (0, Instruction::Ifeq(8)), // branch to 8
///     (3, Instruction::Goto(7)), // unconditional branch to 10 (3 + 7 = 10)
///     (6, Instruction::Ireturn), // unreachable
///     (8, Instruction::Iconst1),
///     (9, Instruction::Ireturn),
///     (10, Instruction::Iconst2),
///     (11, Instruction::Ireturn),
/// ];
/// let blocks = build_basic_blocks(&instructions);
///
/// // Path to PC 10 goes through PC 0, then falls through/jumps to PC 3, which jumps to PC 10.
/// let path = find_shortest_path(&blocks, 0, 10).unwrap();
/// assert_eq!(path, vec![0, 3, 10]);
/// # }
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

    let Ok(start_idx) = blocks.binary_search_by_key(&start_pc, |b| b.start_pc) else {
        return None;
    };
    let Ok(target_idx) = blocks.binary_search_by_key(&target_pc, |b| b.start_pc) else {
        return None;
    };

    let mut visited = vec![false; blocks.len()];
    let mut queue = VecDeque::with_capacity(blocks.len());
    let mut parents = vec![usize::MAX; blocks.len()];

    queue.push_back(start_idx);
    visited[start_idx] = true;

    let mut found = false;

    while let Some(current_idx) = queue.pop_front() {
        if current_idx == target_idx {
            found = true;
            break;
        }

        let block = &blocks[current_idx];
        for next_pc in get_successors(block) {
            if let Ok(next_idx) = blocks.binary_search_by_key(&next_pc, |b| b.start_pc)
                && !visited[next_idx]
            {
                visited[next_idx] = true;
                parents[next_idx] = current_idx;
                queue.push_back(next_idx);
            }
        }
    }

    if found {
        let mut path = Vec::new();
        let mut curr = target_idx;
        while curr != start_idx {
            path.push(blocks[curr].start_pc);
            curr = parents[curr];
        }
        path.push(blocks[start_idx].start_pc);
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
