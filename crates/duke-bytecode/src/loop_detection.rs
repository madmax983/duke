//! Loop Detection on Basic Blocks.
//!
//! This module provides algorithms to identify back-edges and natural loops
//! within a method's control flow graph (CFG). This allows deeper analysis
//! of method behavior, optimizations, and complexity metrics.

#[cfg(feature = "nova")]
use crate::basic_block::BasicBlock;
#[cfg(feature = "nova")]
use crate::reachability::get_successors;
#[cfg(feature = "nova")]
use std::collections::{HashMap, HashSet};

/// Identifies all back-edges in the control flow graph using a Depth-First Search.
///
/// A back-edge is an edge that points to an ancestor node in the DFS traversal tree.
/// Back-edges are the primary indicators of loops in the control flow.
///
/// Returns a list of `(source_pc, target_pc)` representing back-edges.
#[cfg(feature = "nova")]
#[must_use]
pub fn find_back_edges(blocks: &[BasicBlock], entry_pc: usize) -> Vec<(usize, usize)> {
    fn dfs(
        block_start_pc: usize,
        block_map: &HashMap<usize, &BasicBlock>,
        visited: &mut HashSet<usize>,
        stack: &mut HashSet<usize>,
        back_edges: &mut Vec<(usize, usize)>,
    ) {
        visited.insert(block_start_pc);
        stack.insert(block_start_pc);

        if let Some(&block) = block_map.get(&block_start_pc) {
            for next_pc in get_successors(block) {
                // Since branch targets are guaranteed to be leaders, next_pc is a start_pc
                let target_start_pc = next_pc;

                if !visited.contains(&target_start_pc) {
                    dfs(target_start_pc, block_map, visited, stack, back_edges);
                } else if stack.contains(&target_start_pc) {
                    back_edges.push((block_start_pc, target_start_pc));
                }
            }
        }

        stack.remove(&block_start_pc);
    }

    let mut block_map = HashMap::new();
    for block in blocks {
        block_map.insert(block.start_pc, block);
    }

    let mut back_edges = Vec::new();
    let mut visited = HashSet::new();
    let mut stack = HashSet::new(); // nodes currently in the DFS path (ancestors)

    if block_map.contains_key(&entry_pc) {
        dfs(
            entry_pc,
            &block_map,
            &mut visited,
            &mut stack,
            &mut back_edges,
        );
    }

    back_edges
}

/// Identifies natural loops in the control flow graph.
///
/// A natural loop for a given back-edge `(source, target)` (where `target` is the loop header)
/// consists of the `target` plus all nodes that can reach `source` without going through `target`.
///
/// Returns a list of natural loops, where each loop is represented by a vector of `start_pc`s of its basic blocks.
#[cfg(feature = "nova")]
#[must_use]
pub fn find_loops(blocks: &[BasicBlock], entry_pc: usize) -> Vec<Vec<usize>> {
    let back_edges = find_back_edges(blocks, entry_pc);
    let mut loops = Vec::new();

    let mut block_map = HashMap::new();
    // Also build predecessor map for reverse reachability
    let mut predecessors: HashMap<usize, Vec<usize>> = HashMap::new();

    for block in blocks {
        block_map.insert(block.start_pc, block);
        predecessors.entry(block.start_pc).or_default(); // Ensure all blocks are in the map
    }

    for block in blocks {
        for next_pc in get_successors(block) {
            predecessors
                .entry(next_pc)
                .or_default()
                .push(block.start_pc);
        }
    }

    for (source, target) in back_edges {
        let mut loop_nodes = HashSet::new();
        loop_nodes.insert(target); // The header
        loop_nodes.insert(source); // The tail (source of back-edge)

        let mut stack = vec![source];

        // Traverse backwards from source, stopping at target
        while let Some(node) = stack.pop() {
            if let Some(preds) = predecessors.get(&node) {
                for &p in preds {
                    if !loop_nodes.contains(&p) && block_map.contains_key(&p) {
                        loop_nodes.insert(p);
                        stack.push(p);
                    }
                }
            }
        }

        let mut loop_nodes_vec: Vec<usize> = loop_nodes.into_iter().collect();
        loop_nodes_vec.sort_unstable(); // Sort for deterministic output
        loops.push(loop_nodes_vec);
    }

    loops
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
            (4, Instruction::Iload1),   // Loop header
            (5, Instruction::Ifeq(7)),  // Exit loop if 0 (jumps to 12, offset = 12-5=7)
            (8, Instruction::Iconst1),  // Loop body
            (9, Instruction::Goto(-5)), // Back-edge to 4, offset = 4-9=-5
            (12, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);

        let back_edges = find_back_edges(&blocks, 0);
        assert_eq!(back_edges.len(), 1);
        assert_eq!(back_edges[0], (8, 4)); // Block 8 jumps back to Block 4
    }

    #[test]
    fn test_find_loops_simple_loop() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (4, Instruction::Iload1),   // Loop header
            (5, Instruction::Ifeq(7)),  // Exit loop if 0 (jumps to 12)
            (8, Instruction::Iconst1),  // Loop body
            (9, Instruction::Goto(-5)), // Back-edge to 4
            (12, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);

        let loops = find_loops(&blocks, 0);
        assert_eq!(loops.len(), 1);
        let loop_nodes = &loops[0];
        // The loop consists of the header (4) and the body (8)
        assert_eq!(*loop_nodes, vec![4, 8]);
    }

    #[test]
    fn test_no_loops() {
        // Linear code
        let instructions = vec![(0, Instruction::Iconst0), (1, Instruction::Ireturn)];
        let blocks = build_basic_blocks(&instructions);

        let back_edges = find_back_edges(&blocks, 0);
        assert!(back_edges.is_empty());

        let loops = find_loops(&blocks, 0);
        assert!(loops.is_empty());
    }

    #[test]
    fn test_nested_loops() {
        let instructions = vec![
            (0, Instruction::Goto(4)),
            (4, Instruction::Iconst0), // Outer header
            (5, Instruction::Iconst0),
            (8, Instruction::Iconst0),    // Inner header
            (9, Instruction::Ifeq(8)),    // Inner exit -> to 17 (9+8=17)
            (12, Instruction::Goto(-4)),  // Inner back-edge to 8
            (17, Instruction::Goto(-13)), // Outer back-edge to 4
            (21, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);

        let mut back_edges = find_back_edges(&blocks, 0);
        back_edges.sort_unstable();

        assert_eq!(back_edges, vec![(12, 8), (17, 4)]);

        let mut loops = find_loops(&blocks, 0);
        loops.sort();

        assert_eq!(loops[0], vec![4, 8, 12, 17]);
        assert_eq!(loops[1], vec![8, 12]);
    }
}
