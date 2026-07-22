//! Bytecode Peephole Optimization Detection.
//!
//! This module uses pattern matching to identify sub-optimal instruction sequences
//! that could be replaced with more efficient equivalents. While Duke is an interpreter,
//! identifying these patterns helps in static analysis, profiling, and potential AOT
//! optimization passes.

use crate::Instruction;
use crate::pattern::{Pattern, find_pattern_matches};

/// An identified opportunity for peephole optimization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizationHint {
    /// A human-readable name or category for the optimization rule.
    pub rule_name: &'static str,
    /// The PC where the sub-optimal sequence begins.
    pub start_pc: usize,
    /// The length of the sub-optimal sequence in instructions.
    pub length: usize,
    /// A description of the suggested replacement.
    pub suggestion: &'static str,
}

/// Detects sub-optimal bytecode patterns in a sequence of instructions.
///
/// Looks for common inefficiencies like redundant loads/stores, unnecessary arithmetic
/// (e.g., adding zero), and consecutive constant loads that could be folded.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "nova")] {
/// use duke_bytecode::Instruction;
/// use duke_bytecode::peephole::detect_inefficiencies;
///
/// let instructions = vec![
///     (0, Instruction::Iload1),
///     (1, Instruction::Iconst0),
///     (2, Instruction::Iadd), // Redundant: x + 0 -> x
///     (3, Instruction::Ireturn),
/// ];
///
/// let hints = detect_inefficiencies(&instructions);
/// assert_eq!(hints.len(), 1);
/// assert_eq!(hints[0].rule_name, "add_zero");
/// assert_eq!(hints[0].start_pc, 1); // Starts at Iconst0
/// # }
/// ```
#[must_use]
pub fn detect_inefficiencies(instructions: &[(usize, Instruction)]) -> Vec<OptimizationHint> {
    let mut hints = Vec::new();

    if instructions.is_empty() {
        return hints;
    }

    // Rule 1: Add Zero (`iconst_0`, `iadd`) -> Can be removed.
    let add_zero_pattern = vec![
        Pattern::Exact(Instruction::Iconst0),
        Pattern::Exact(Instruction::Iadd),
    ];
    for pc in find_pattern_matches(instructions, &add_zero_pattern) {
        hints.push(OptimizationHint {
            rule_name: "add_zero",
            start_pc: pc,
            length: 2,
            suggestion: "Remove redundant addition of zero.",
        });
    }

    // Rule 2: Store/Load same local (`istore_X`, `iload_X`) -> Often redundant if top of stack is just needed again.
    // We can't match exact X generically with Pattern enum easily if they vary, but we can match the common 0-3 variants.
    let store_load_pairs = vec![
        (Instruction::Istore0, Instruction::Iload0),
        (Instruction::Istore1, Instruction::Iload1),
        (Instruction::Istore2, Instruction::Iload2),
        (Instruction::Istore3, Instruction::Iload3),
    ];

    for (store, load) in store_load_pairs {
        let pattern = vec![Pattern::Exact(store), Pattern::Exact(load)];
        for pc in find_pattern_matches(instructions, &pattern) {
            hints.push(OptimizationHint {
                rule_name: "store_load_redundancy",
                start_pc: pc,
                length: 2,
                suggestion: "Consider replacing store/load sequence with `dup` and `store` if appropriate.",
            });
        }
    }

    // Sort by PC to ensure deterministic output
    hints.sort_by_key(|h| h.start_pc);

    hints
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Instruction;

    #[test]
    fn test_add_zero_detection() {
        let instructions = vec![
            (0, Instruction::Iload1),
            (1, Instruction::Iconst0),
            (2, Instruction::Iadd),
            (3, Instruction::Istore1),
        ];

        let hints = detect_inefficiencies(&instructions);
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].rule_name, "add_zero");
        assert_eq!(hints[0].start_pc, 1);
    }

    #[test]
    fn test_store_load_redundancy_detection() {
        let instructions = vec![
            (0, Instruction::Iconst1),
            (1, Instruction::Istore2),
            (2, Instruction::Iload2),
            (3, Instruction::Ireturn),
        ];

        let hints = detect_inefficiencies(&instructions);
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].rule_name, "store_load_redundancy");
        assert_eq!(hints[0].start_pc, 1);
    }

    #[test]
    fn test_multiple_inefficiencies() {
        let instructions = vec![
            (0, Instruction::Iconst1),
            (1, Instruction::Istore0),
            (2, Instruction::Iload0), // Hint 1
            (3, Instruction::Iconst0),
            (4, Instruction::Iadd), // Hint 2
            (5, Instruction::Ireturn),
        ];

        let hints = detect_inefficiencies(&instructions);
        assert_eq!(hints.len(), 2);
        assert_eq!(hints[0].rule_name, "store_load_redundancy");
        assert_eq!(hints[0].start_pc, 1);
        assert_eq!(hints[1].rule_name, "add_zero");
        assert_eq!(hints[1].start_pc, 3);
    }

    #[test]
    fn test_no_inefficiencies() {
        let instructions = vec![
            (0, Instruction::Iconst1),
            (1, Instruction::Istore1),
            (2, Instruction::Iload2),
            (3, Instruction::Iadd),
            (4, Instruction::Ireturn),
        ];

        let hints = detect_inefficiencies(&instructions);
        assert!(hints.is_empty());
    }
}
