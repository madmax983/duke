//! Loop detection for basic blocks.
//!
//! This module provides algorithms for detecting back-edges in a control flow graph,
//! which indicate the presence of loops (e.g. while/for/do-while structures).

#[cfg(feature = "nova")]
use crate::basic_block::BasicBlock;
#[cfg(feature = "nova")]
use crate::reachability::get_successors;
#[cfg(feature = "nova")]
use std::collections::{HashMap, HashSet};

/// Finds all back-edges in a control flow graph of basic blocks.
///
/// A back-edge is an edge `(u, v)` where `v` is an ancestor of `u` in the depth-first search tree.
///
/// Returns a list of `(from_pc, to_pc)` tuples.
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

    let mut back_edges = Vec::new();
    let mut visited = HashSet::with_capacity(blocks.len());
    let mut rec_stack = HashSet::with_capacity(blocks.len());

    dfs(
        entry_pc,
        &block_map,
        &mut visited,
        &mut rec_stack,
        &mut back_edges,
    );

    // Sort to ensure deterministic output
    back_edges.sort_unstable();
    back_edges
}

#[cfg(feature = "nova")]
fn dfs(
    pc: usize,
    block_map: &HashMap<usize, &BasicBlock>,
    visited: &mut HashSet<usize>,
    rec_stack: &mut HashSet<usize>,
    back_edges: &mut Vec<(usize, usize)>,
) {
    if !visited.insert(pc) {
        return;
    }
    rec_stack.insert(pc);

    if let Some(block) = block_map.get(&pc) {
        let successors = get_successors(block);
        for next_pc in successors {
            if !block_map.contains_key(&next_pc) {
                continue;
            }

            if rec_stack.contains(&next_pc) {
                // Back-edge found
                back_edges.push((pc, next_pc));
            } else if !visited.contains(&next_pc) {
                dfs(next_pc, block_map, visited, rec_stack, back_edges);
            }
        }
    }

    rec_stack.remove(&pc);
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
            // Loop start
            (2, Instruction::Iload1),
            (3, Instruction::Iconst5),
            (4, Instruction::IfIcmpge(11)), // Exit loop -> pc 15
            (7, Instruction::Iinc { index: 1, value: 1 }),
            (10, Instruction::Goto(-8)), // Back-edge -> pc 2
            // Loop end
            (13, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);

        let back_edges = find_back_edges(&blocks, 0);
        assert_eq!(back_edges.len(), 1);
        // The block containing the back-edge instruction (10: Goto) starts at PC 7
        assert_eq!(back_edges[0], (7, 2));
    }

    #[test]
    fn test_find_back_edges_no_loops() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Ifeq(5)),
            (4, Instruction::Iconst1),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst2),
            (7, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);

        let back_edges = find_back_edges(&blocks, 0);
        assert!(back_edges.is_empty());
    }
}
