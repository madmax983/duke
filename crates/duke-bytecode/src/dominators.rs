//! Dominator Tree Analysis.
//!
//! This module computes the dominators for basic blocks in a Control Flow Graph (CFG).
//! A block A dominates block B if every path from the entry node to B must go through A.

#[cfg(feature = "nova")]
use crate::basic_block::BasicBlock;
#[cfg(feature = "nova")]
use crate::reachability::get_successors;
#[cfg(feature = "nova")]
use std::collections::{HashMap, HashSet};

/// Computes the dominators for a given set of basic blocks.
///
/// Returns a map from each block's `start_pc` to a set of `start_pc`s of the blocks that dominate it.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "nova")] {
/// use duke_bytecode::Instruction;
/// use duke_bytecode::build_basic_blocks;
/// use duke_bytecode::compute_dominators;
///
/// let instructions = vec![
///     (0, Instruction::Ifeq(6)), // branch to 6
///     (3, Instruction::Iconst1),
///     (4, Instruction::Goto(2)), // goto +2 = 6
///     (6, Instruction::Iconst2),
///     (7, Instruction::Ireturn),
/// ];
/// let blocks = build_basic_blocks(&instructions);
///
/// let doms = compute_dominators(&blocks, 0);
///
/// // Block 0 dominates everything
/// assert!(doms.get(&6).is_some_and(|d| d.contains(&0)));
/// # }
/// ```
#[cfg(feature = "nova")]
#[must_use]
pub fn compute_dominators(
    blocks: &[BasicBlock],
    entry_pc: usize,
) -> HashMap<usize, HashSet<usize>> {
    if blocks.is_empty() {
        return HashMap::new();
    }

    let mut all_pcs = HashSet::with_capacity(blocks.len());
    let mut preds: HashMap<usize, Vec<usize>> = HashMap::with_capacity(blocks.len());

    for block in blocks {
        all_pcs.insert(block.start_pc);
        preds.entry(block.start_pc).or_default(); // Ensure every block has an entry
    }

    for block in blocks {
        for succ_pc in get_successors(block) {
            let mut target_pc = None;
            for b in blocks {
                if succ_pc >= b.start_pc && succ_pc < b.end_pc {
                    target_pc = Some(b.start_pc);
                    break;
                }
            }
            if target_pc.is_none() && all_pcs.contains(&succ_pc) {
                target_pc = Some(succ_pc);
            }
            if target_pc.is_none() {
                for b in blocks {
                    if b.start_pc == succ_pc {
                        target_pc = Some(b.start_pc);
                        break;
                    }
                }
            }

            if let Some(pc) = target_pc {
                preds.entry(pc).or_default().push(block.start_pc);
            }
        }
    }

    // Now manually fix up fallthrough edges for basic blocks that don't end in unconditional jumps/returns
    for i in 0..blocks.len() - 1 {
        let block = &blocks[i];
        let next_block = &blocks[i + 1];

        let mut falls_through = false;
        if let Some((_, last_instr)) = block.instructions.last() {
            if !last_instr.is_unconditional_jump() && !last_instr.is_return() {
                falls_through = true;
            }
        } else {
            falls_through = true; // empty block falls through
        }

        if falls_through {
            let p = preds.entry(next_block.start_pc).or_default();
            if !p.contains(&block.start_pc) {
                p.push(block.start_pc);
            }
        }
    }

    let mut doms: HashMap<usize, HashSet<usize>> = HashMap::with_capacity(blocks.len());

    // Initialize: Dom(entry) = {entry}, Dom(others) = all nodes
    for &pc in &all_pcs {
        if pc == entry_pc {
            let mut set = HashSet::new();
            set.insert(pc);
            doms.insert(pc, set);
        } else {
            doms.insert(pc, all_pcs.clone());
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for &pc in &all_pcs {
            if pc == entry_pc {
                continue;
            }

            let mut new_dom: Option<HashSet<usize>> = None;

            if let Some(predecessors) = preds.get(&pc) {
                for &pred_pc in predecessors {
                    if let Some(pred_doms) = doms.get(&pred_pc) {
                        if let Some(ref mut nd) = new_dom {
                            nd.retain(|x| pred_doms.contains(x));
                        } else {
                            new_dom = Some(pred_doms.clone());
                        }
                    }
                }
            }

            let mut new_dom = new_dom.unwrap_or_default();
            new_dom.insert(pc);

            if let Some(current_dom) = doms.get(&pc)
                && *current_dom != new_dom
            {
                doms.insert(pc, new_dom);
                changed = true;
            }
        }
    }

    doms
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
            (0, Instruction::Ifeq(6)),
            (3, Instruction::Iconst1),
            (4, Instruction::Goto(3)), // PC=4, target=7 (4+3)
            (6, Instruction::Iconst2), // PC=6
            (7, Instruction::Ireturn), // PC=7
        ];
        let blocks = build_basic_blocks(&instructions);

        let doms = compute_dominators(&blocks, 0);

        // Blocks should be at 0, 3, 6, 7
        assert!(doms.get(&0).is_some_and(|d| d.contains(&0)));
        assert!(doms.get(&3).is_some_and(|d| d.contains(&0)));
        assert!(doms.get(&6).is_some_and(|d| d.contains(&0)));
        assert!(doms.get(&7).is_some_and(|d| d.contains(&0)));

        assert!(doms.get(&7).is_some_and(|d| d.contains(&7)));
    }
}
