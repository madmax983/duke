//! Bytecode Pattern Matching Analysis.
//!
//! This module provides utilities for finding specific sequences of instructions
//! within a method's bytecode. This is foundational for static analysis, peephole
//! optimization detection, and security linting.

#[cfg(feature = "nova")]
use crate::Instruction;
#[cfg(feature = "nova")]
use std::mem::discriminant;

/// A pattern to match against a single JVM instruction.
#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
pub enum Pattern {
    /// Matches the instruction exactly, including all operands.
    Exact(Instruction),
    /// Matches the instruction's opcode (variant), ignoring any operands.
    Variant(Instruction),
    /// Matches any instruction.
    Any,
}

#[cfg(feature = "nova")]
impl Pattern {
    /// Checks if this pattern matches the given instruction.
    #[must_use]
    pub fn matches(&self, instr: &Instruction) -> bool {
        match self {
            Self::Exact(exact_instr) => exact_instr == instr,
            Self::Variant(variant_instr) => discriminant(variant_instr) == discriminant(instr),
            Self::Any => true,
        }
    }
}

/// Finds all non-overlapping occurrences of a pattern sequence in the given bytecode.
///
/// Returns a list of the starting PCs (Program Counters) where the pattern sequence begins.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "nova")] {
/// use duke_bytecode::Instruction;
/// use duke_bytecode::pattern::{Pattern, find_pattern_matches};
///
/// let instructions = vec![
///     (0, Instruction::Iload1),
///     (1, Instruction::Iconst1),
///     (2, Instruction::Iadd),
///     (3, Instruction::Ireturn),
/// ];
///
/// // Find occurrences of `iconst_1` followed by any instruction, then `ireturn`.
/// let pattern = vec![
///     Pattern::Exact(Instruction::Iconst1),
///     Pattern::Any,
///     Pattern::Variant(Instruction::Ireturn),
/// ];
///
/// let matches = find_pattern_matches(&instructions, &pattern);
/// assert_eq!(matches, vec![1]);
/// # }
/// ```
#[cfg(feature = "nova")]
#[must_use]
pub fn find_pattern_matches(
    instructions: &[(usize, Instruction)],
    pattern: &[Pattern],
) -> Vec<usize> {
    if pattern.is_empty() || instructions.len() < pattern.len() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let mut i = 0;

    while i <= instructions.len() - pattern.len() {
        let mut all_matched = true;
        for (j, pat) in pattern.iter().enumerate() {
            if !pat.matches(&instructions[i + j].1) {
                all_matched = false;
                break;
            }
        }

        if all_matched {
            matches.push(instructions[i].0);
            i += pattern.len(); // non-overlapping
        } else {
            i += 1;
        }
    }

    matches
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::Instruction;

    #[test]
    fn test_exact_match() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Iload1),
            (2, Instruction::Iadd),
        ];

        let pattern = vec![
            Pattern::Exact(Instruction::Iconst0),
            Pattern::Exact(Instruction::Iload1),
        ];

        let matches = find_pattern_matches(&instructions, &pattern);
        assert_eq!(matches, vec![0]);
    }

    #[test]
    fn test_variant_match() {
        let instructions = vec![
            (0, Instruction::Iload(5)), // Different operand!
            (2, Instruction::Ireturn),
        ];

        let pattern = vec![
            Pattern::Variant(Instruction::Iload(0)), // Should match any Iload
            Pattern::Variant(Instruction::Ireturn),
        ];

        let matches = find_pattern_matches(&instructions, &pattern);
        assert_eq!(matches, vec![0]);
    }

    #[test]
    fn test_any_match() {
        let instructions = vec![
            (0, Instruction::Nop),
            (1, Instruction::Iconst0),
            (2, Instruction::Ireturn),
        ];

        let pattern = vec![Pattern::Any, Pattern::Any];

        // Non-overlapping matches: (0, 1), (2, .. wait, only 1 full match of len 2 before we have 1 left)
        let matches = find_pattern_matches(&instructions, &pattern);
        assert_eq!(matches, vec![0]);
    }

    #[test]
    fn test_no_match() {
        let instructions = vec![(0, Instruction::Iconst0), (1, Instruction::Ireturn)];

        let pattern = vec![Pattern::Exact(Instruction::Iconst1)];

        let matches = find_pattern_matches(&instructions, &pattern);
        assert!(matches.is_empty());
    }
}
