//! Basic Block Clone Detection.
//!
//! This module combines basic block analysis with bytecode similarity
//! to find redundant or duplicated code segments within a method.

#[cfg(feature = "nova")]
use crate::basic_block::BasicBlock;
#[cfg(feature = "nova")]
use crate::similarity::calculate_similarity;

/// Represents a detected clone between two basic blocks.
#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq)]
pub struct CloneReport {
    /// Start PC of the first basic block.
    pub block_a_start_pc: usize,
    /// Start PC of the second basic block.
    pub block_b_start_pc: usize,
    /// Similarity score between 0.0 and 1.0.
    pub similarity_score: f64,
}

/// Scans a list of basic blocks for structural clones.
///
/// Compares every unique pair of blocks. If their similarity score
/// meets or exceeds the given `threshold`, they are reported as clones.
/// Tiny blocks (fewer than 2 instructions) are ignored to prevent noise.
#[cfg(feature = "nova")]
#[must_use]
pub fn detect_clones(blocks: &[BasicBlock], threshold: f64) -> Vec<CloneReport> {
    let mut reports = Vec::new();

    for i in 0..blocks.len() {
        for j in (i + 1)..blocks.len() {
            let b1 = &blocks[i];
            let b2 = &blocks[j];

            // Ignore tiny blocks
            if b1.instructions.len() < 2 || b2.instructions.len() < 2 {
                continue;
            }

            let seq1: Vec<_> = b1
                .instructions
                .iter()
                .map(|(_, inst)| inst.clone())
                .collect();
            let seq2: Vec<_> = b2
                .instructions
                .iter()
                .map(|(_, inst)| inst.clone())
                .collect();

            let score = calculate_similarity(&seq1, &seq2);
            if score >= threshold {
                reports.push(CloneReport {
                    block_a_start_pc: b1.start_pc,
                    block_b_start_pc: b2.start_pc,
                    similarity_score: score,
                });
            }
        }
    }

    reports
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::Instruction;
    use crate::basic_block::BasicBlock;

    #[test]
    fn test_detect_clones() {
        let b1 = BasicBlock {
            start_pc: 0,
            end_pc: 3,
            instructions: vec![
                (0, Instruction::Iload(1)),
                (1, Instruction::Istore(2)),
                (2, Instruction::Ireturn),
            ],
        };

        let b2 = BasicBlock {
            start_pc: 3,
            end_pc: 6,
            instructions: vec![
                (3, Instruction::Iload(3)),
                (4, Instruction::Istore(4)),
                (5, Instruction::Ireturn),
            ],
        };

        let blocks = vec![b1, b2];
        let clones = detect_clones(&blocks, 0.9);

        assert_eq!(clones.len(), 1);
        assert_eq!(clones[0].block_a_start_pc, 0);
        assert_eq!(clones[0].block_b_start_pc, 3);
        assert!((clones[0].similarity_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_ignore_tiny_blocks() {
        let b1 = BasicBlock {
            start_pc: 0,
            end_pc: 1,
            instructions: vec![(0, Instruction::Ireturn)],
        };
        let b2 = BasicBlock {
            start_pc: 1,
            end_pc: 2,
            instructions: vec![(1, Instruction::Ireturn)],
        };

        let blocks = vec![b1, b2];
        let clones = detect_clones(&blocks, 0.9);
        assert!(clones.is_empty());
    }
}
