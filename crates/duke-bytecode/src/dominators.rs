//! Dominator tree and loop detection analysis.
//!
//! This module provides algorithms to compute dominators for basic blocks
//! and identify natural loops via back-edges.

#[cfg(feature = "nova")]
use crate::basic_block::BasicBlock;
#[cfg(feature = "nova")]
use crate::reachability::get_successors;
#[cfg(feature = "nova")]
use std::collections::{HashMap, HashSet};

/// Computes the dominator sets for a list of basic blocks.
///
/// A block `D` dominates block `N` if every path from the entry block to `N` must go through `D`.
#[cfg(feature = "nova")]
#[must_use]
pub fn compute_dominators(blocks: &[BasicBlock], entry_pc: usize) -> HashMap<usize, HashSet<usize>> {
    let mut doms: HashMap<usize, HashSet<usize>> = HashMap::new();
    if blocks.is_empty() {
        return doms;
    }

    let all_pcs: HashSet<usize> = blocks.iter().map(|b| b.start_pc).collect();

    for block in blocks {
        if block.start_pc == entry_pc {
            let mut set = HashSet::new();
            set.insert(entry_pc);
            doms.insert(entry_pc, set);
        } else {
            doms.insert(block.start_pc, all_pcs.clone());
        }
    }

    // Pre-compute predecessors
    let mut preds_map: HashMap<usize, Vec<usize>> = HashMap::with_capacity(blocks.len());
    for block in blocks {
        for succ in get_successors(block) {
            preds_map.entry(succ).or_default().push(block.start_pc);
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for block in blocks {
            if block.start_pc == entry_pc { continue; }

            let empty_preds = Vec::new();
            let preds = preds_map.get(&block.start_pc).unwrap_or(&empty_preds);

            let mut new_dom = if preds.is_empty() {
                HashSet::new()
            } else {
                let mut isect = all_pcs.clone(); // start with all
                for p in preds {
                    if let Some(p_dom) = doms.get(p) {
                        isect = isect.intersection(p_dom).copied().collect();
                    }
                }
                isect
            };

            new_dom.insert(block.start_pc);

            if doms.get(&block.start_pc) != Some(&new_dom) {
                doms.insert(block.start_pc, new_dom);
                changed = true;
            }
        }
    }

    doms
}

/// Identifies natural loops in the basic blocks by finding back-edges.
///
/// A back-edge is an edge A -> B where B dominates A.
/// Returns a list of loops, where each loop is represented by its header PC and the set of PCs in the loop.
#[cfg(feature = "nova")]
#[must_use]
#[allow(clippy::collapsible_if)]
pub fn find_natural_loops(blocks: &[BasicBlock], entry_pc: usize) -> Vec<(usize, HashSet<usize>)> {
    let doms = compute_dominators(blocks, entry_pc);
    let mut loops = Vec::new();

    for block in blocks {
        let succs = get_successors(block);
        for succ in succs {
            // Is it a back-edge? block -> succ, and succ dominates block
            if let Some(dom_set) = doms.get(&block.start_pc) {
                if dom_set.contains(&succ) {
                    let mut loop_nodes = HashSet::new();
                    loop_nodes.insert(succ);
                    loop_nodes.insert(block.start_pc);

                    let mut stack = vec![block.start_pc];
                    while let Some(node) = stack.pop() {
                        if node == succ {
                            continue;
                        }
                        for p in blocks {
                            if get_successors(p).contains(&node) && !loop_nodes.contains(&p.start_pc) {
                                loop_nodes.insert(p.start_pc);
                                stack.push(p.start_pc);
                            }
                        }
                    }
                    // Sort loops by header PC to be deterministic
                    loops.push((succ, loop_nodes));
                }
            }
        }
    }

    loops.sort_by_key(|(header, _)| *header);
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
            (0, Instruction::Iload0),
            (1, Instruction::Ifeq(5)), // branch to 6
            (4, Instruction::Iconst1),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst2),
            (7, Instruction::Ireturn),
        ];
        let blocks = build_basic_blocks(&instructions);
        let doms = compute_dominators(&blocks, 0);

        // Block 0 dominates everything
        for b in &blocks {
            assert!(doms.get(&b.start_pc).unwrap().contains(&0));
        }
    }


    #[test]
    fn test_find_natural_loops() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Goto(-1)), // jump back to 0
        ];
        let blocks = build_basic_blocks(&instructions);
        let loops = find_natural_loops(&blocks, 0);

        assert_eq!(loops.len(), 1);
        let (header, nodes) = &loops[0];
        assert_eq!(*header, 0);
        assert!(nodes.contains(&0)); // 0 is the only block start_pc in this CFG
    }
}
