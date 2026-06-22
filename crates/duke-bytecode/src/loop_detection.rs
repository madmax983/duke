//! Loop Detection Analysis.
//!
//! This module provides algorithms to identify cyclical control flow
//! within a method's basic blocks by finding back-edges.

#[cfg(feature = "nova")]
use crate::basic_block::BasicBlock;
#[cfg(feature = "nova")]
use crate::reachability::get_successors;
#[cfg(feature = "nova")]
use std::collections::{HashMap, HashSet};

/// Represents a back-edge in the control flow graph, indicating a loop.
#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BackEdge {
    /// The program counter (PC) where the loop jumps back from.
    pub source_pc: usize,
    /// The program counter (PC) that is the target of the backward jump (loop header).
    pub target_pc: usize,
}

/// Finds all back-edges in the basic block control flow graph using Depth-First Search.
///
/// A back-edge is an edge `u -> v` where `v` is an ancestor of `u` in the DFS tree.
/// These edges represent loops in the bytecode.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "nova")] {
/// use duke_bytecode::Instruction;
/// use duke_bytecode::{build_basic_blocks, BackEdge, find_back_edges};
///
/// let instructions = vec![
///     (0, Instruction::Iconst0),
///     (1, Instruction::Istore1),
///     (2, Instruction::Iload1), // Loop header
///     (3, Instruction::Iconst5),
///     (4, Instruction::IfIcmpge(11)), // Exit loop if i >= 5
///     (7, Instruction::Iinc { index: 1, value: 1 }),
///     (10, Instruction::Goto(2)), // Back-edge to 2
///     (11, Instruction::Ireturn),
/// ];
/// let blocks = build_basic_blocks(&instructions);
///
/// let back_edges = find_back_edges(&blocks, 0);
/// assert_eq!(back_edges.len(), 1);
/// assert_eq!(back_edges[0].source_pc, 7); // The block ending at Goto(2) starts at 7
/// assert_eq!(back_edges[0].target_pc, 2);
/// # }
/// ```
#[cfg(feature = "nova")]
#[must_use]
pub fn find_back_edges(blocks: &[BasicBlock], entry_pc: usize) -> Vec<BackEdge> {
    if blocks.is_empty() {
        return Vec::new();
    }

    let mut block_map = HashMap::with_capacity(blocks.len());
    for block in blocks {
        block_map.insert(block.start_pc, block);
    }

    let mut visited = HashSet::with_capacity(blocks.len());
    let mut recursion_stack = HashSet::with_capacity(blocks.len());
    let mut back_edges = Vec::new();

    dfs(
        entry_pc,
        &block_map,
        &mut visited,
        &mut recursion_stack,
        &mut back_edges,
    );

    back_edges
}

#[cfg(feature = "nova")]
fn dfs(
    current_pc: usize,
    block_map: &HashMap<usize, &BasicBlock>,
    visited: &mut HashSet<usize>,
    recursion_stack: &mut HashSet<usize>,
    back_edges: &mut Vec<BackEdge>,
) {
    if !block_map.contains_key(&current_pc) {
        return;
    }

    visited.insert(current_pc);
    recursion_stack.insert(current_pc);

    if let Some(block) = block_map.get(&current_pc) {
        for next_pc in get_successors(block) {
            if !visited.contains(&next_pc) {
                dfs(next_pc, block_map, visited, recursion_stack, back_edges);
            } else if recursion_stack.contains(&next_pc) {
                back_edges.push(BackEdge {
                    source_pc: current_pc,
                    target_pc: next_pc,
                });
            }
        }
    }

    recursion_stack.remove(&current_pc);
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::Instruction;
    use crate::basic_block::build_basic_blocks;

    #[test]
    fn test_find_back_edges_simple_loop() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Istore1),
            (2, Instruction::Iload1), // target of goto
            (3, Instruction::Iconst5),
            (4, Instruction::IfIcmpge(11)), // branch to end
            (7, Instruction::Iinc { index: 1, value: 1 }),
            (10, Instruction::Goto(2)), // jump back to 2
            (11, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);

        let edges = find_back_edges(&blocks, 0);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].source_pc, 7); // The block starting at 7 ends with Goto(2)
        assert_eq!(edges[0].target_pc, 2);
    }

    #[test]
    fn test_find_back_edges_no_loops() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Ifeq(5)),
            (4, Instruction::Ireturn),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);

        let edges = find_back_edges(&blocks, 0);
        assert!(edges.is_empty());
    }

    #[test]
    fn test_find_back_edges_infinite_loop() {
        let instructions = vec![
            (0, Instruction::Nop),
            (1, Instruction::Goto(1)), // loops to itself
        ];
        let blocks = build_basic_blocks(&instructions);

        let edges = find_back_edges(&blocks, 0);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].source_pc, 1);
        assert_eq!(edges[0].target_pc, 1);
    }
}
