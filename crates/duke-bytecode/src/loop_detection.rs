//! Loop detection and back-edge analysis.
//!
//! This module provides algorithms to identify loops within a method's control flow graph
//! by finding back-edges using Depth-First Search (DFS).

#[cfg(feature = "nova")]
use crate::basic_block::BasicBlock;
#[cfg(feature = "nova")]
use crate::reachability::get_successors;

use std::collections::{HashMap, HashSet};

/// Finds all back-edges in a method's control flow graph.
///
/// A back-edge is defined as an edge `(u, v)` where `v` is an ancestor of `u` in the DFS traversal.
/// This is typically indicative of a loop (e.g., `while`, `for` loops in source code).
/// Returns a list of `(source_pc, target_pc)` representing the back-edges.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "nova")] {
/// use duke_bytecode::Instruction;
/// use duke_bytecode::{BasicBlock, build_basic_blocks};
/// use duke_bytecode::find_back_edges;
///
/// // Simulated while(true) loop
/// let instructions = vec![(0, Instruction::Goto(0))];
/// let blocks = build_basic_blocks(&instructions);
///
/// let back_edges = find_back_edges(&blocks, 0);
/// assert_eq!(back_edges, vec![(0, 0)]);
/// # }
/// ```
#[cfg(feature = "nova")]
#[must_use]
pub fn find_back_edges(blocks: &[BasicBlock], entry_pc: usize) -> Vec<(usize, usize)> {
    if blocks.is_empty() {
        return Vec::new();
    }

    let mut block_map = HashMap::with_capacity(blocks.len());
    for block in blocks {
        block_map.insert(block.start_pc, block);
    }

    let mut visited = HashSet::with_capacity(blocks.len());
    let mut rec_stack = HashSet::with_capacity(blocks.len());
    let mut back_edges = Vec::new();

    dfs(
        entry_pc,
        &block_map,
        &mut visited,
        &mut rec_stack,
        &mut back_edges,
    );

    // Also look for disconnected components that might be loops
    for block in blocks {
        if !visited.contains(&block.start_pc) {
            dfs(
                block.start_pc,
                &block_map,
                &mut visited,
                &mut rec_stack,
                &mut back_edges,
            );
        }
    }

    // Sort to make the output deterministic
    back_edges.sort_unstable();
    back_edges
}

#[cfg(feature = "nova")]
fn dfs(
    u: usize,
    block_map: &HashMap<usize, &BasicBlock>,
    visited: &mut HashSet<usize>,
    rec_stack: &mut HashSet<usize>,
    back_edges: &mut Vec<(usize, usize)>,
) {
    visited.insert(u);
    rec_stack.insert(u);

    if let Some(block) = block_map.get(&u) {
        for v in get_successors(block) {
            // Find the actual start_pc of the block that this target 'v' falls into.
            let next_pc = if block_map.contains_key(&v) {
                v
            } else {
                // Find block containing v
                block_map
                    .values()
                    .find(|b| b.start_pc == v || (v > b.start_pc && v < b.end_pc))
                    .map_or(v, |b| b.start_pc)
            };

            if !visited.contains(&next_pc) {
                dfs(next_pc, block_map, visited, rec_stack, back_edges);
            } else if rec_stack.contains(&next_pc) {
                // Push (source_block_start, target_block_start)
                back_edges.push((u, next_pc));
            }
        }
    }

    rec_stack.remove(&u);
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
            // Loop start (PC 2)
            (2, Instruction::Iinc { index: 1, value: 1 }),
            (5, Instruction::Iload1),
            (6, Instruction::Iconst5),
            (7, Instruction::IfIcmplt(-5)), // branch back to PC 2 (-5 offset from 7)
            // Loop end
            (10, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);

        let back_edges = find_back_edges(&blocks, 0);
        // We expect a back edge from the block starting at 2 (which contains the branch back to PC 2) to PC 2.
        assert_eq!(back_edges, vec![(2, 2)]);
    }

    #[test]
    fn test_find_back_edges_no_loop() {
        let instructions = vec![(0, Instruction::Iconst0), (1, Instruction::Ireturn)];
        let blocks = build_basic_blocks(&instructions);

        let back_edges = find_back_edges(&blocks, 0);
        assert!(back_edges.is_empty());
    }
}
