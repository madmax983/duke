//! Bytecode Similarity Analysis.
//!
//! This module provides algorithms to calculate the similarity between two sequences
//! of JVM bytecode instructions.

#[cfg(feature = "nova")]
use crate::Instruction;
#[cfg(feature = "nova")]
use std::mem::discriminant;

/// Calculates the similarity score between two bytecode instruction sequences.
///
/// Returns a score between 0.0 (completely different) and 1.0 (identical).
/// The algorithm computes the Levenshtein distance based on instruction variants
/// (ignoring operands) to find structural clones.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "nova")] {
/// use duke_bytecode::Instruction;
/// use duke_bytecode::calculate_similarity;
///
/// let seq1 = vec![Instruction::Iload(1), Instruction::Istore(2), Instruction::Ireturn];
/// let seq2 = vec![Instruction::Iload(3), Instruction::Istore(4), Instruction::Ireturn];
///
/// // Same structural flow, different constants/locals
/// assert!((calculate_similarity(&seq1, &seq2) - 1.0).abs() < f64::EPSILON);
/// # }
/// ```
#[cfg(feature = "nova")]
#[must_use]
pub fn calculate_similarity(seq1: &[Instruction], seq2: &[Instruction]) -> f64 {
    let len1 = seq1.len();
    let len2 = seq2.len();

    if len1 == 0 && len2 == 0 {
        return 1.0;
    }
    if len1 == 0 || len2 == 0 {
        return 0.0;
    }

    let mut dp = vec![vec![0; len2 + 1]; len1 + 1];

    for (i, row) in dp.iter_mut().enumerate().take(len1 + 1) {
        row[0] = i;
    }
    for (j, item) in dp[0].iter_mut().enumerate().take(len2 + 1) {
        *item = j;
    }

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = usize::from(discriminant(&seq1[i - 1]) != discriminant(&seq2[j - 1]));

            dp[i][j] = std::cmp::min(
                std::cmp::min(dp[i - 1][j] + 1, dp[i][j - 1] + 1),
                dp[i - 1][j - 1] + cost,
            );
        }
    }

    #[allow(clippy::cast_precision_loss)]
    let max_len = std::cmp::max(len1, len2) as f64;
    #[allow(clippy::cast_precision_loss)]
    let distance = dp[len1][len2] as f64;

    1.0 - (distance / max_len)
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::Instruction;

    #[test]
    fn test_identical_sequences() {
        let seq1 = vec![Instruction::Iconst0, Instruction::Ireturn];
        let seq2 = vec![Instruction::Iconst0, Instruction::Ireturn];
        assert!((calculate_similarity(&seq1, &seq2) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_completely_different() {
        let seq1 = vec![Instruction::Iconst0, Instruction::Ireturn];
        let seq2 = vec![Instruction::Nop, Instruction::Nop, Instruction::Nop];
        assert!((calculate_similarity(&seq1, &seq2) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_structural_clones() {
        let seq1 = vec![
            Instruction::Iload(1),
            Instruction::Istore(2),
            Instruction::Ireturn,
        ];
        let seq2 = vec![
            Instruction::Iload(3),
            Instruction::Istore(4),
            Instruction::Ireturn,
        ];
        assert!((calculate_similarity(&seq1, &seq2) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_partial_match() {
        let seq1 = vec![Instruction::Iconst0, Instruction::Ireturn];
        let seq2 = vec![Instruction::Iconst0, Instruction::Nop, Instruction::Ireturn];
        let sim = calculate_similarity(&seq1, &seq2);
        assert!(sim > 0.0 && sim < 1.0);
    }

    #[test]
    fn test_empty_sequences() {
        let seq1: Vec<Instruction> = vec![];
        let seq2: Vec<Instruction> = vec![];
        assert!((calculate_similarity(&seq1, &seq2) - 1.0).abs() < f64::EPSILON);

        let seq3 = vec![Instruction::Ireturn];
        assert!((calculate_similarity(&seq1, &seq3) - 0.0).abs() < f64::EPSILON);
    }
}
