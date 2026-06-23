//! Natural Loop Detection.
//!
//! This module provides algorithms for detecting natural loops in control flow graphs
//! represented by basic blocks.

#[cfg(feature = "nova")]
use std::collections::{HashMap, HashSet};

#[cfg(feature = "nova")]
use crate::BasicBlock;
#[cfg(feature = "nova")]
use crate::reachability::get_successors;

/// Computes the dominators for each basic block.
///
/// A block `d` dominates a block `n` if every path from the entry node to `n` must go through `d`.
/// By definition, every node dominates itself.
///
/// Returns a map from block `start_pc` to a set of block `start_pc`s that dominate it.
///
/// # Panics
///
/// Panics if a node has an empty list of predecessors but was reported to have one (should not happen for valid CFGs).
#[cfg(feature = "nova")]
#[must_use]
pub fn compute_dominators(
    blocks: &[BasicBlock],
    entry_pc: usize,
) -> HashMap<usize, HashSet<usize>> {
    let mut doms: HashMap<usize, HashSet<usize>> = HashMap::with_capacity(blocks.len());
    if blocks.is_empty() {
        return doms;
    }

    let all_nodes: HashSet<usize> = blocks.iter().map(|b| b.start_pc).collect();
    let mut predecessors: HashMap<usize, HashSet<usize>> = HashMap::with_capacity(blocks.len());
    for block in blocks {
        predecessors.entry(block.start_pc).or_default();
    }
    for block in blocks {
        for succ in get_successors(block) {
            predecessors.entry(succ).or_default().insert(block.start_pc);
        }
    }

    // Initialize dominators
    for &node in &all_nodes {
        if node == entry_pc {
            let mut set = HashSet::new();
            set.insert(entry_pc);
            doms.insert(node, set);
        } else {
            doms.insert(node, all_nodes.clone());
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for node in &all_nodes {
            if *node == entry_pc {
                continue;
            }

            let default_preds = HashSet::new();
            let preds = predecessors.get(node).unwrap_or(&default_preds);
            let mut new_dom = if preds.is_empty() {
                HashSet::new()
            } else {
                let mut iter = preds.iter();
                let first_pred = iter.next().unwrap();
                let mut intersection = doms
                    .get(first_pred)
                    .cloned()
                    .unwrap_or_else(|| all_nodes.clone());
                for pred in iter {
                    let pred_doms = doms.get(pred).cloned().unwrap_or_else(|| all_nodes.clone());
                    intersection.retain(|d| pred_doms.contains(d));
                }
                intersection
            };

            new_dom.insert(*node);

            let current_dom = doms.get(node).cloned().unwrap_or_else(|| all_nodes.clone());
            if new_dom != current_dom {
                doms.insert(*node, new_dom);
                changed = true;
            }
        }
    }

    doms
}

/// Finds natural loops in the control flow graph.
///
/// A natural loop is defined by a back edge (a sequence from node `n` to node `h`,
/// where `h` dominates `n`). `h` is the loop header. The natural loop consists of `h`
/// and all nodes that can reach `n` without going through `h`.
///
/// Returns a map from the loop header (`start_pc`) to a set of block `start_pc`s that belong to the loop.
#[cfg(feature = "nova")]
#[must_use]
pub fn find_natural_loops(
    blocks: &[BasicBlock],
    entry_pc: usize,
) -> HashMap<usize, HashSet<usize>> {
    let mut loops: HashMap<usize, HashSet<usize>> = HashMap::new();
    if blocks.is_empty() {
        return loops;
    }

    let doms = compute_dominators(blocks, entry_pc);
    let mut predecessors: HashMap<usize, HashSet<usize>> = HashMap::with_capacity(blocks.len());
    for block in blocks {
        predecessors.entry(block.start_pc).or_default();
    }
    for block in blocks {
        for succ in get_successors(block) {
            predecessors.entry(succ).or_default().insert(block.start_pc);
        }
    }

    // Find back edges (n -> h) where h dominates n
    for block in blocks {
        let n = block.start_pc;
        for h in get_successors(block) {
            let is_back_edge = doms.get(&n).is_some_and(|dom_n| dom_n.contains(&h));
            if is_back_edge {
                // We found a back edge n -> h. This defines a natural loop with header h.
                let loop_nodes = loops.entry(h).or_default();
                loop_nodes.insert(h);
                loop_nodes.insert(n);

                // If n != h, find all nodes that can reach n without going through h.
                if n != h {
                    let mut stack = vec![n];
                    while let Some(m) = stack.pop() {
                        if let Some(preds) = predecessors.get(&m) {
                            for &p in preds {
                                if loop_nodes.insert(p) {
                                    stack.push(p);
                                }
                            }
                        }
                    }
                }
            }
        }
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
    fn test_compute_dominators() {
        let instructions = vec![
            (0, Instruction::Ifeq(6)), // branch to 6 (1 + 6 = 7? No! Wait! Ifeq takes an offset! 0 + 6 = 6.)
            (3, Instruction::Iconst1),
            (4, Instruction::Goto(5)), // goto offset 5 (4 + 5 = 9)
            (6, Instruction::Iconst2),
            (7, Instruction::Nop), // fallthrough to 9
            (9, Instruction::Ireturn),
        ];
        // Blocks:
        // 0: [0] -> 3, 6
        // 3: [3, 4] -> 9
        // 6: [6, 7] -> 9
        // 9: [9] -> return
        let blocks = build_basic_blocks(&instructions);

        let doms = compute_dominators(&blocks, 0);

        assert!(doms.get(&0).unwrap().contains(&0));
        assert!(doms.get(&3).unwrap().contains(&0));
        assert!(doms.get(&3).unwrap().contains(&3));
        assert!(doms.get(&6).unwrap().contains(&0));
        assert!(doms.get(&6).unwrap().contains(&6));
        // 9 is dominated by 0 (and itself) because path can go 0->3->9 or 0->6->9
        assert!(doms.get(&9).unwrap().contains(&0));
        assert!(doms.get(&9).unwrap().contains(&9));
        assert!(!doms.get(&9).unwrap().contains(&3));
        assert!(!doms.get(&9).unwrap().contains(&6));
    }

    #[test]
    fn test_find_natural_loops() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Istore1),
            // Loop header
            (2, Instruction::Iload1),
            (3, Instruction::Iconst5),
            (4, Instruction::IfIcmpge(8)), // exit loop if >= 5 (4 + 8 = 12)
            (7, Instruction::Iinc { index: 1, value: 1 }),
            (10, Instruction::Goto(-8)), // back edge to 2 (10 - 8 = 2)
            // Loop exit
            (12, Instruction::Ireturn),
        ];
        // Blocks:
        // 0: [0, 1] -> 2
        // 2: [2, 3, 4] -> 7, 12
        // 7: [7, 10] -> 2
        // 12: [12] -> return

        let blocks = build_basic_blocks(&instructions);
        let loops = find_natural_loops(&blocks, 0);

        // Loop header is 2
        assert_eq!(loops.len(), 1);
        let loop_nodes = loops.get(&2).unwrap();
        assert!(loop_nodes.contains(&2));
        assert!(loop_nodes.contains(&7));
        assert!(!loop_nodes.contains(&0));
        assert!(!loop_nodes.contains(&12));
    }
}
