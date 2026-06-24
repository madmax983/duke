//! Basic Block Loop Detection.
//!
//! This module provides algorithms to identify loops within basic blocks
//! of JVM bytecode.

#[cfg(feature = "nova")]
use crate::basic_block::BasicBlock;
#[cfg(feature = "nova")]
use crate::reachability::get_successors;
#[cfg(feature = "nova")]
use std::collections::{HashMap, HashSet};

/// Represents a loop detected in the basic block control flow graph.
#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loop {
    /// The program counter (PC) of the basic block that serves as the loop header.
    pub header_pc: usize,
    /// The program counters (PCs) of the basic blocks that jump back to the header (back-edges).
    pub back_edge_pcs: Vec<usize>,
}

/// Identifies natural loops in a control flow graph of basic blocks using DFS.
/// A back-edge is identified when a successor is currently on the DFS recursion stack.
///
/// Returns a list of `Loop` structures.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "nova")] {
/// use duke_bytecode::Instruction;
/// use duke_bytecode::{BasicBlock, build_basic_blocks};
/// use duke_bytecode::loop_detection::find_loops;
///
/// // A simple loop: 0 -> 2 -> 7 -> 2 -> 7 -> ...
/// let instructions = vec![
///     (0, Instruction::Iconst0),
///     (1, Instruction::Istore1),
///     // Loop Header
///     (2, Instruction::Iload1),
///     (3, Instruction::Iconst5),
///     (4, Instruction::IfIcmpge(7)), // 4 + 7 = 11 (Exit loop)
///     // Loop Body
///     (7, Instruction::Iinc { index: 1, value: 1 }),
///     (10, Instruction::Goto(-8)), // 10 - 8 = 2 (Back edge)
///     (11, Instruction::Ireturn),
/// ];
/// let blocks = build_basic_blocks(&instructions);
///
/// let loops = find_loops(&blocks, 0);
/// assert_eq!(loops.len(), 1);
/// assert_eq!(loops[0].header_pc, 2);
/// assert_eq!(loops[0].back_edge_pcs, vec![7]); // block 7 jumps to 2
/// # }
/// ```
#[cfg(feature = "nova")]
#[must_use]
pub fn find_loops(blocks: &[BasicBlock], entry_pc: usize) -> Vec<Loop> {
    if blocks.is_empty() {
        return Vec::new();
    }

    let mut block_map = HashMap::with_capacity(blocks.len());
    for block in blocks {
        block_map.insert(block.start_pc, block);
    }

    let mut visited = HashSet::with_capacity(blocks.len());
    let mut rec_stack = HashSet::with_capacity(blocks.len());
    let mut back_edges = HashMap::new();

    dfs(
        entry_pc,
        &block_map,
        &mut visited,
        &mut rec_stack,
        &mut back_edges,
    );

    let mut loops = Vec::new();
    for (header, mut tails) in back_edges {
        tails.sort_unstable();
        loops.push(Loop {
            header_pc: header,
            back_edge_pcs: tails,
        });
    }

    loops.sort_by_key(|l| l.header_pc);
    loops
}

#[cfg(feature = "nova")]
fn dfs(
    current: usize,
    block_map: &HashMap<usize, &BasicBlock>,
    visited: &mut HashSet<usize>,
    rec_stack: &mut HashSet<usize>,
    back_edges: &mut HashMap<usize, Vec<usize>>,
) {
    visited.insert(current);
    rec_stack.insert(current);

    if let Some(block) = block_map.get(&current) {
        for next in get_successors(block) {
            if !visited.contains(&next) {
                dfs(next, block_map, visited, rec_stack, back_edges);
            } else if rec_stack.contains(&next) {
                // Back edge found
                back_edges.entry(next).or_default().push(current);
            }
        }
    }

    rec_stack.remove(&current);
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::Instruction;
    use crate::basic_block::build_basic_blocks;

    #[test]
    fn test_find_simple_loop() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            // Header
            (1, Instruction::Iload1),
            (2, Instruction::Ifeq(7)), // 2 + 5 = 7
            // Body
            (5, Instruction::Goto(-4)), // back edge 5 - 4 = 1
            // Exit
            (7, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);

        let loops = find_loops(&blocks, 0);
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].header_pc, 1);
        assert_eq!(loops[0].back_edge_pcs, vec![5]);
    }

    #[test]
    fn test_no_loops() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Ifeq(5)),
            (4, Instruction::Iconst1),
            (5, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);

        let loops = find_loops(&blocks, 0);
        assert!(loops.is_empty());
    }
}
