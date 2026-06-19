//! Bytecode Statistics Analysis.
//!
//! This module provides tools to analyze and gather statistical
//! metrics from a sequence of JVM bytecode instructions, such as
//! instruction frequencies and basic block lengths.

#[cfg(feature = "nova")]
use crate::Instruction;
#[cfg(feature = "nova")]
use std::collections::HashMap;

/// Calculates the frequency of each instruction mnemonic in the given bytecode sequence.
///
/// This provides a basic statistical overview of the operations performed
/// within a method, which can be useful for profiling or identifying
/// specific code patterns (e.g., heavy math vs. heavy memory access).
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "nova")] {
/// use duke_bytecode::Instruction;
/// use duke_bytecode::instruction_frequencies;
///
/// let seq = vec![
///     (0, Instruction::Iload(1)),
///     (1, Instruction::Istore(2)),
///     (2, Instruction::Iload(1)),
///     (3, Instruction::Ireturn)
/// ];
///
/// let freqs = instruction_frequencies(&seq);
/// assert_eq!(freqs.get("iload"), Some(&2));
/// assert_eq!(freqs.get("istore"), Some(&1));
/// assert_eq!(freqs.get("ireturn"), Some(&1));
/// # }
/// ```
#[cfg(feature = "nova")]
#[must_use]
pub fn instruction_frequencies(instructions: &[(usize, Instruction)]) -> HashMap<&'static str, usize> {
    let mut freqs = HashMap::new();
    for (_, instr) in instructions {
        *freqs.entry(instr.mnemonic()).or_insert(0) += 1;
    }
    freqs
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::Instruction;

    #[test]
    fn test_instruction_frequencies() {
        let seq = vec![
            (0, Instruction::Iload(1)),
            (1, Instruction::Istore(2)),
            (2, Instruction::Iload(1)),
            (3, Instruction::Ireturn),
            (4, Instruction::Nop),
            (5, Instruction::Nop),
        ];

        let freqs = instruction_frequencies(&seq);
        assert_eq!(freqs.get("iload"), Some(&2));
        assert_eq!(freqs.get("istore"), Some(&1));
        assert_eq!(freqs.get("ireturn"), Some(&1));
        assert_eq!(freqs.get("nop"), Some(&2));
        assert_eq!(freqs.get("aload"), None);
    }

    #[test]
    fn test_empty_sequence() {
        let seq: Vec<(usize, Instruction)> = vec![];
        let freqs = instruction_frequencies(&seq);
        assert!(freqs.is_empty());
    }
}
