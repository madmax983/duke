//! Structural diffing of basic blocks.
//!
//! This module provides functionality to compare two sequences of basic blocks,
//! determining which blocks were added, removed, modified, or remain identical.
//! This gives semantic meaning to changes rather than simple bytecode hash diffs.

use crate::basic_block::BasicBlock;

/// Represents the structural difference between basic blocks.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BlockDiff {
    /// The block is identical in both sequences.
    Identical { block: BasicBlock },
    /// The block exists in the new sequence but not the old.
    Added { block: BasicBlock },
    /// The block exists in the old sequence but not the new.
    Removed { block: BasicBlock },
    /// The block exists in both but its instructions have changed.
    Modified {
        old_block: BasicBlock,
        new_block: BasicBlock,
    },
}

/// Compares two sequences of basic blocks to produce a sequence of block differences.
///
/// This simple algorithm attempts to align blocks by their `start_pc`.
/// For a more robust diff, sequence alignment algorithms (like Myers Diff) could be used.
#[must_use]
pub fn compare_blocks(old_blocks: &[BasicBlock], new_blocks: &[BasicBlock]) -> Vec<BlockDiff> {
    let mut diffs = Vec::new();

    // Create maps from start_pc to Block for quick lookup
    let mut old_map = std::collections::HashMap::new();
    for block in old_blocks {
        old_map.insert(block.start_pc, block);
    }

    let mut new_map = std::collections::HashMap::new();
    for block in new_blocks {
        new_map.insert(block.start_pc, block);
    }

    // To ensure a somewhat ordered output, gather all unique start PCs
    let mut all_pcs: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    all_pcs.extend(old_map.keys());
    all_pcs.extend(new_map.keys());

    for pc in all_pcs {
        match (old_map.get(&pc), new_map.get(&pc)) {
            (Some(old_b), Some(new_b)) => {
                // Same start PC, compare contents
                if old_b.instructions == new_b.instructions {
                    diffs.push(BlockDiff::Identical {
                        block: (*old_b).clone(),
                    });
                } else {
                    diffs.push(BlockDiff::Modified {
                        old_block: (*old_b).clone(),
                        new_block: (*new_b).clone(),
                    });
                }
            }
            (Some(old_b), None) => {
                diffs.push(BlockDiff::Removed {
                    block: (*old_b).clone(),
                });
            }
            (None, Some(new_b)) => {
                diffs.push(BlockDiff::Added {
                    block: (*new_b).clone(),
                });
            }
            (None, None) => unreachable!(),
        }
    }

    diffs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::Instruction;

    #[test]
    fn test_compare_blocks_identical() {
        let b1 = BasicBlock {
            start_pc: 0,
            end_pc: 2,
            instructions: vec![(0, Instruction::Iconst0)],
        };
        let b2 = BasicBlock {
            start_pc: 0,
            end_pc: 2,
            instructions: vec![(0, Instruction::Iconst0)],
        };
        let diffs = compare_blocks(std::slice::from_ref(&b1), std::slice::from_ref(&b2));
        assert_eq!(diffs.len(), 1);
        assert!(matches!(diffs[0], BlockDiff::Identical { .. }));
    }

    #[test]
    fn test_compare_blocks_added() {
        let b1 = BasicBlock {
            start_pc: 0,
            end_pc: 2,
            instructions: vec![(0, Instruction::Iconst0)],
        };
        let b2 = BasicBlock {
            start_pc: 2,
            end_pc: 4,
            instructions: vec![(2, Instruction::Ireturn)],
        };
        let diffs = compare_blocks(std::slice::from_ref(&b1), &[b1.clone(), b2]);
        assert_eq!(diffs.len(), 2);

        let has_added = diffs.iter().any(|d| matches!(d, BlockDiff::Added { .. }));
        assert!(has_added, "Expected an Added diff");
    }

    #[test]
    fn test_compare_blocks_removed() {
        let b1 = BasicBlock {
            start_pc: 0,
            end_pc: 2,
            instructions: vec![(0, Instruction::Iconst0)],
        };
        let b2 = BasicBlock {
            start_pc: 2,
            end_pc: 4,
            instructions: vec![(2, Instruction::Ireturn)],
        };
        let diffs = compare_blocks(&[b1.clone(), b2], std::slice::from_ref(&b1));
        assert_eq!(diffs.len(), 2);

        let has_removed = diffs.iter().any(|d| matches!(d, BlockDiff::Removed { .. }));
        assert!(has_removed, "Expected a Removed diff");
    }

    #[test]
    fn test_compare_blocks_modified() {
        let b1 = BasicBlock {
            start_pc: 0,
            end_pc: 2,
            instructions: vec![(0, Instruction::Iconst0)],
        };
        let b2 = BasicBlock {
            start_pc: 0,
            end_pc: 2,
            instructions: vec![(0, Instruction::Iconst1)],
        };
        let diffs = compare_blocks(&[b1], &[b2]);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(diffs[0], BlockDiff::Modified { .. }));
    }
}
