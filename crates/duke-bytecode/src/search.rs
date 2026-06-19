//! Bytecode Pattern Searcher.
//!
//! This module provides capabilities to search through a sequence of JVM instructions
//! for specific structural patterns, abstracting over exact index values or constants.

#[cfg(feature = "nova")]
use crate::Instruction;

/// Represents a matching pattern for a JVM instruction.
/// This allows for exact matches or wildcard matches for instruction categories.
#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    /// Matches exactly the specified instruction.
    Exact(Instruction),
    /// Matches any instruction.
    Any,
    /// Matches any local variable load instruction (e.g., `Iload`, `Iload0`, `IloadW`, `Lload`, etc.).
    AnyLoad,
    /// Matches any local variable store instruction (e.g., `Istore`, `Istore0`, `IstoreW`, `Lstore`, etc.).
    AnyStore,
    /// Matches any constant push instruction (e.g., `Iconst0`, `Ldc`, `Sipush`, etc.).
    AnyConst,
}

#[cfg(feature = "nova")]
impl Pattern {
    /// Checks if a given instruction matches this pattern.
    #[must_use]
    pub fn matches(&self, instr: &Instruction) -> bool {
        match self {
            Self::Exact(exact_instr) => exact_instr == instr,
            Self::Any => true,
            Self::AnyLoad => matches!(
                instr,
                Instruction::Iload(_)
                    | Instruction::Iload0
                    | Instruction::Iload1
                    | Instruction::Iload2
                    | Instruction::Iload3
                    | Instruction::IloadW(_)
                    | Instruction::Lload(_)
                    | Instruction::Lload0
                    | Instruction::Lload1
                    | Instruction::Lload2
                    | Instruction::Lload3
                    | Instruction::LloadW(_)
                    | Instruction::Fload(_)
                    | Instruction::Fload0
                    | Instruction::Fload1
                    | Instruction::Fload2
                    | Instruction::Fload3
                    | Instruction::FloadW(_)
                    | Instruction::Dload(_)
                    | Instruction::Dload0
                    | Instruction::Dload1
                    | Instruction::Dload2
                    | Instruction::Dload3
                    | Instruction::DloadW(_)
                    | Instruction::Aload(_)
                    | Instruction::Aload0
                    | Instruction::Aload1
                    | Instruction::Aload2
                    | Instruction::Aload3
                    | Instruction::AloadW(_)
            ),
            Self::AnyStore => matches!(
                instr,
                Instruction::Istore(_)
                    | Instruction::Istore0
                    | Instruction::Istore1
                    | Instruction::Istore2
                    | Instruction::Istore3
                    | Instruction::IstoreW(_)
                    | Instruction::Lstore(_)
                    | Instruction::Lstore0
                    | Instruction::Lstore1
                    | Instruction::Lstore2
                    | Instruction::Lstore3
                    | Instruction::LstoreW(_)
                    | Instruction::Fstore(_)
                    | Instruction::Fstore0
                    | Instruction::Fstore1
                    | Instruction::Fstore2
                    | Instruction::Fstore3
                    | Instruction::FstoreW(_)
                    | Instruction::Dstore(_)
                    | Instruction::Dstore0
                    | Instruction::Dstore1
                    | Instruction::Dstore2
                    | Instruction::Dstore3
                    | Instruction::DstoreW(_)
                    | Instruction::Astore(_)
                    | Instruction::Astore0
                    | Instruction::Astore1
                    | Instruction::Astore2
                    | Instruction::Astore3
                    | Instruction::AstoreW(_)
            ),
            Self::AnyConst => matches!(
                instr,
                Instruction::AconstNull
                    | Instruction::IconstM1
                    | Instruction::Iconst0
                    | Instruction::Iconst1
                    | Instruction::Iconst2
                    | Instruction::Iconst3
                    | Instruction::Iconst4
                    | Instruction::Iconst5
                    | Instruction::Lconst0
                    | Instruction::Lconst1
                    | Instruction::Fconst0
                    | Instruction::Fconst1
                    | Instruction::Fconst2
                    | Instruction::Dconst0
                    | Instruction::Dconst1
                    | Instruction::Bipush(_)
                    | Instruction::Sipush(_)
                    | Instruction::Ldc(_)
                    | Instruction::LdcW(_)
                    | Instruction::Ldc2W(_)
            ),
        }
    }
}

/// Searches for the first occurrence of a sequence of patterns within a bytecode slice.
///
/// Returns the starting index in the slice where the pattern matches, or `None` if not found.
/// Note that the index returned is the index into the provided `instructions` slice, not the `pc`.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "nova")] {
/// use duke_bytecode::Instruction;
/// use duke_bytecode::{find_sequence, Pattern};
///
/// let instructions = vec![
///     (0, Instruction::Iconst1),
///     (1, Instruction::Iload(2)),
///     (3, Instruction::Iadd),
///     (4, Instruction::Istore(3)),
/// ];
///
/// // Find a pattern: [AnyLoad, Iadd, AnyStore]
/// let pattern = vec![
///     Pattern::AnyLoad,
///     Pattern::Exact(Instruction::Iadd),
///     Pattern::AnyStore,
/// ];
///
/// let match_idx = find_sequence(&instructions, &pattern);
/// assert_eq!(match_idx, Some(1));
/// # }
/// ```
#[cfg(feature = "nova")]
#[must_use]
pub fn find_sequence(instructions: &[(usize, Instruction)], pattern: &[Pattern]) -> Option<usize> {
    if pattern.is_empty() || instructions.len() < pattern.len() {
        return None;
    }

    for i in 0..=(instructions.len() - pattern.len()) {
        let mut matched = true;
        for (j, pat) in pattern.iter().enumerate() {
            if !pat.matches(&instructions[i + j].1) {
                matched = false;
                break;
            }
        }
        if matched {
            return Some(i);
        }
    }

    None
}

/// Searches for all occurrences of a sequence of patterns within a bytecode slice.
///
/// Returns a vector of starting indices in the slice where the pattern matches.
/// Note that the indices returned are the indices into the provided `instructions` slice.
#[cfg(feature = "nova")]
#[must_use]
pub fn find_all_sequences(
    instructions: &[(usize, Instruction)],
    pattern: &[Pattern],
) -> Vec<usize> {
    let mut results = Vec::new();

    if pattern.is_empty() || instructions.len() < pattern.len() {
        return results;
    }

    let mut i = 0;
    while i <= instructions.len() - pattern.len() {
        let mut matched = true;
        for (j, pat) in pattern.iter().enumerate() {
            if !pat.matches(&instructions[i + j].1) {
                matched = false;
                break;
            }
        }
        if matched {
            results.push(i);
            i += pattern.len(); // Skip ahead to avoid overlapping matches
        } else {
            i += 1;
        }
    }

    results
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::Instruction;

    #[test]
    fn test_pattern_matching_exact() {
        let pat = Pattern::Exact(Instruction::Iconst1);
        assert!(pat.matches(&Instruction::Iconst1));
        assert!(!pat.matches(&Instruction::Iconst0));
    }

    #[test]
    fn test_pattern_matching_any() {
        let pat = Pattern::Any;
        assert!(pat.matches(&Instruction::Iconst1));
        assert!(pat.matches(&Instruction::Nop));
        assert!(pat.matches(&Instruction::Iload(5)));
    }

    #[test]
    fn test_pattern_matching_any_load() {
        let pat = Pattern::AnyLoad;
        assert!(pat.matches(&Instruction::Iload0));
        assert!(pat.matches(&Instruction::Aload1));
        assert!(pat.matches(&Instruction::Lload(3)));
        assert!(!pat.matches(&Instruction::Istore0));
        assert!(!pat.matches(&Instruction::Iconst0));
    }

    #[test]
    fn test_pattern_matching_any_store() {
        let pat = Pattern::AnyStore;
        assert!(pat.matches(&Instruction::Istore0));
        assert!(pat.matches(&Instruction::Astore1));
        assert!(pat.matches(&Instruction::Lstore(3)));
        assert!(!pat.matches(&Instruction::Iload0));
        assert!(!pat.matches(&Instruction::Iconst0));
    }

    #[test]
    fn test_pattern_matching_any_const() {
        let pat = Pattern::AnyConst;
        assert!(pat.matches(&Instruction::Iconst0));
        assert!(pat.matches(&Instruction::Bipush(10)));
        assert!(pat.matches(&Instruction::AconstNull));
        assert!(!pat.matches(&Instruction::Iload0));
        assert!(!pat.matches(&Instruction::Iadd));
    }

    #[test]
    fn test_find_sequence() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Iconst1),
            (2, Instruction::Iload0),
            (3, Instruction::Iadd),
            (4, Instruction::Istore1),
        ];

        let pat1 = vec![
            Pattern::Exact(Instruction::Iconst1),
            Pattern::AnyLoad,
            Pattern::Exact(Instruction::Iadd),
        ];

        assert_eq!(find_sequence(&instructions, &pat1), Some(1));

        let pat2 = vec![Pattern::AnyLoad, Pattern::Exact(Instruction::Isub)];
        assert_eq!(find_sequence(&instructions, &pat2), None);
    }

    #[test]
    fn test_find_all_sequences() {
        let instructions = vec![
            (0, Instruction::Iload0),
            (1, Instruction::Istore1),
            (2, Instruction::Iconst0),
            (3, Instruction::Iload2),
            (4, Instruction::Istore3),
        ];

        let pat = vec![Pattern::AnyLoad, Pattern::AnyStore];

        let matches = find_all_sequences(&instructions, &pat);
        assert_eq!(matches, vec![0, 3]);
    }
}
