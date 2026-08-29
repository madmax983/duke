//! Dominator tree analysis for basic blocks.
//!
//! This module provides algorithms to compute dominators and immediate dominators
//! for a control flow graph of basic blocks.

#[cfg(feature = "nova")]
use crate::basic_block::BasicBlock;
#[cfg(feature = "nova")]
use std::collections::{HashMap, HashSet};

/// Computes the dominator sets for all basic blocks in the control flow graph.
///
/// A block `d` dominates a block `n` if every path from the entry node to `n`
/// must go through `d`. By definition, every block dominates itself.
///
/// Returns a map from `start_pc` to a set of `start_pc`s that dominate it.
///
/// # Panics
///
/// Panics if the internal graph traversal fails to find predecessors for a node.
#[cfg(feature = "nova")]
#[must_use]
pub fn compute_dominators(
    blocks: &[BasicBlock],
    entry_pc: usize,
) -> HashMap<usize, HashSet<usize>> {
    if blocks.is_empty() {
        return HashMap::new();
    }

    let mut all_nodes = HashSet::new();
    let mut preds: HashMap<usize, Vec<usize>> = HashMap::new();

    for block in blocks {
        all_nodes.insert(block.start_pc);
        preds.entry(block.start_pc).or_default();
        let successors = crate::reachability::get_successors(block);
        for succ in successors {
            preds.entry(succ).or_default().push(block.start_pc);
        }
    }

    let mut dominators: HashMap<usize, HashSet<usize>> = HashMap::new();
    dominators.insert(entry_pc, {
        let mut set = HashSet::new();
        set.insert(entry_pc);
        set
    });

    for node in &all_nodes {
        if *node != entry_pc {
            dominators.insert(*node, all_nodes.clone());
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for node in &all_nodes {
            if *node == entry_pc {
                continue;
            }

            let mut new_dom: Option<HashSet<usize>> = None;
            for p in preds.get(node).unwrap() {
                if let Some(p_dom) = dominators.get(p) {
                    if let Some(ref mut nd) = new_dom {
                        *nd = nd.intersection(p_dom).copied().collect();
                    } else {
                        new_dom = Some(p_dom.clone());
                    }
                }
            }

            let mut new_dom = new_dom.unwrap_or_default();
            new_dom.insert(*node);

            if dominators.get(node).unwrap() != &new_dom {
                dominators.insert(*node, new_dom);
                changed = true;
            }
        }
    }

    dominators
}

/// Computes the immediate dominator for each basic block in the control flow graph.
///
/// The immediate dominator or idom of a node `n` is the unique node that strictly
/// dominates `n` but does not strictly dominate any other node that strictly dominates `n`.
/// Every node except the entry node has a unique immediate dominator.
///
/// Returns a map from `start_pc` to the `start_pc` of its immediate dominator.
#[cfg(feature = "nova")]
#[must_use]
pub fn compute_immediate_dominators(
    blocks: &[BasicBlock],
    entry_pc: usize,
) -> HashMap<usize, usize> {
    if blocks.is_empty() {
        return HashMap::new();
    }

    let dominators = compute_dominators(blocks, entry_pc);
    let mut idoms = HashMap::new();

    for (node, doms) in &dominators {
        if *node == entry_pc {
            continue;
        }

        // Strict dominators: all dominators except the node itself
        let strict_doms: HashSet<usize> = doms.iter().copied().filter(|&d| d != *node).collect();

        let mut idom = None;
        for &s_dom in &strict_doms {
            let mut is_idom = true;
            for &other_s_dom in &strict_doms {
                if s_dom != other_s_dom
                    && let Some(other_doms) = dominators.get(&other_s_dom)
                    && other_doms.contains(&s_dom)
                {
                    is_idom = false;
                    break;
                }
            }
            if is_idom {
                idom = Some(s_dom);
                break;
            }
        }

        if let Some(id) = idom {
            idoms.insert(*node, id);
        }
    }

    idoms
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::Instruction;
    use crate::basic_block::build_basic_blocks;

    #[test]
    fn test_linear_cfg() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Istore1),
            (2, Instruction::Iconst1),
            (3, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);
        let doms = compute_dominators(&blocks, 0);

        assert_eq!(doms.len(), 1);
        assert!(doms.get(&0).unwrap().contains(&0));
    }

    #[test]
    fn test_branching_cfg() {
        // 0 -> 4, 6
        // 4 -> 7
        // 6 -> 7
        let instructions = vec![
            (0, Instruction::Ifeq(6)), // Branch to 6
            (3, Instruction::Goto(4)), // Jump to 7 (3+4=7)
            (6, Instruction::Iconst1), // Target 1
            (7, Instruction::Ireturn), // Target 2 (merge)
        ];
        let blocks = build_basic_blocks(&instructions);
        let doms = compute_dominators(&blocks, 0);

        assert_eq!(doms.len(), 4);

        // 0 dominates everything
        assert!(doms.get(&0).unwrap().contains(&0));
        assert!(doms.get(&3).unwrap().contains(&0));
        assert!(doms.get(&6).unwrap().contains(&0));
        assert!(doms.get(&7).unwrap().contains(&0));

        // 7 should only be dominated by 0 and 7
        assert!(doms.get(&7).unwrap().contains(&7));
        assert!(!doms.get(&7).unwrap().contains(&3));
        assert!(!doms.get(&7).unwrap().contains(&6));
    }

    #[test]
    fn test_compute_immediate_dominators() {
        let instructions = vec![
            (0, Instruction::Ifeq(6)), // Branch to 6
            (3, Instruction::Goto(4)), // Jump to 7
            (6, Instruction::Iconst1),
            (7, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);
        let idoms = compute_immediate_dominators(&blocks, 0);

        assert_eq!(idoms.len(), 3); // Entry node doesn't have an idom
        assert_eq!(idoms.get(&3), Some(&0));
        assert_eq!(idoms.get(&6), Some(&0));
        assert_eq!(idoms.get(&7), Some(&0));
    }
}
