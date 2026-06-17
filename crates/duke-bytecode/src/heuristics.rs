//! Method Pattern Heuristics Analysis.
//!
//! This module analyzes a sequence of bytecode instructions to determine
//! high-level structural patterns, such as whether a method is a simple
//! getter, setter, or if it allocates memory.

#[cfg(feature = "nova")]
use crate::Instruction;

/// High-level patterns detected in a method's bytecode.
#[cfg(feature = "nova")]
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MethodPattern {
    /// True if the method is a simple getter (e.g. `aload_0`, `getfield`, `return`).
    pub is_getter: bool,
    /// True if the method is a simple setter (e.g. `aload_0`, `*load_*`, `putfield`, `return`).
    pub is_setter: bool,
    /// True if the method does nothing but return.
    pub is_empty: bool,
    /// True if the method contains instructions that allocate memory (`new`, `newarray`, etc.).
    pub allocates_memory: bool,
    /// True if the method contains backwards branches, indicating a loop.
    pub has_loops: bool,
}

/// Analyzes a sequence of instructions to identify method heuristics.
#[cfg(feature = "nova")]
#[must_use]
pub fn analyze_method_patterns(instructions: &[(usize, Instruction)]) -> MethodPattern {
    let mut pattern = MethodPattern::default();

    if instructions.is_empty() {
        return pattern;
    }

    if instructions.len() == 1 && matches!(instructions[0].1, Instruction::Return) {
        pattern.is_empty = true;
    }

    // Check for getter
    if instructions.len() == 3
        && matches!(instructions[0].1, Instruction::Aload0)
        && matches!(instructions[1].1, Instruction::Getfield(_))
        && is_return_instr(&instructions[2].1)
    {
        pattern.is_getter = true;
    }

    // Check for setter
    if instructions.len() == 4
        && matches!(instructions[0].1, Instruction::Aload0)
        && is_load_instr(&instructions[1].1)
        && matches!(instructions[2].1, Instruction::Putfield(_))
        && matches!(instructions[3].1, Instruction::Return)
    {
        pattern.is_setter = true;
    }

    // Check for allocations and loops
    for (pc, instr) in instructions {
        if is_allocation_instr(instr) {
            pattern.allocates_memory = true;
        }

        // Loop detection: any backwards branch
        if instr.is_conditional_branch() || instr.is_unconditional_jump() {
            let targets = instr.control_flow_targets(*pc, None);
            for target in targets {
                if target <= *pc {
                    pattern.has_loops = true;
                }
            }
        }
    }

    pattern
}

#[cfg(feature = "nova")]
const fn is_return_instr(instr: &Instruction) -> bool {
    matches!(
        instr,
        Instruction::Ireturn
            | Instruction::Lreturn
            | Instruction::Freturn
            | Instruction::Dreturn
            | Instruction::Areturn
            | Instruction::Return
    )
}

#[cfg(feature = "nova")]
const fn is_load_instr(instr: &Instruction) -> bool {
    matches!(
        instr,
        Instruction::Iload(_)
            | Instruction::Lload(_)
            | Instruction::Fload(_)
            | Instruction::Dload(_)
            | Instruction::Aload(_)
            | Instruction::Iload0
            | Instruction::Iload1
            | Instruction::Iload2
            | Instruction::Iload3
            | Instruction::Lload0
            | Instruction::Lload1
            | Instruction::Lload2
            | Instruction::Lload3
            | Instruction::Fload0
            | Instruction::Fload1
            | Instruction::Fload2
            | Instruction::Fload3
            | Instruction::Dload0
            | Instruction::Dload1
            | Instruction::Dload2
            | Instruction::Dload3
            | Instruction::Aload0
            | Instruction::Aload1
            | Instruction::Aload2
            | Instruction::Aload3
    )
}

#[cfg(feature = "nova")]
const fn is_allocation_instr(instr: &Instruction) -> bool {
    matches!(
        instr,
        Instruction::New(_)
            | Instruction::Newarray(_)
            | Instruction::Anewarray(_)
            | Instruction::Multianewarray { .. }
    )
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::Instruction;

    #[test]
    fn test_is_empty_method() {
        let instrs = vec![(0, Instruction::Return)];
        let pat = analyze_method_patterns(&instrs);
        assert!(pat.is_empty);
        assert!(!pat.is_getter);
    }

    #[test]
    fn test_is_getter_method() {
        let instrs = vec![
            (0, Instruction::Aload0),
            (1, Instruction::Getfield(duke_classfile::CpIndex(42))),
            (4, Instruction::Ireturn),
        ];
        let pat = analyze_method_patterns(&instrs);
        assert!(pat.is_getter);
        assert!(!pat.is_setter);
    }

    #[test]
    fn test_is_setter_method() {
        let instrs = vec![
            (0, Instruction::Aload0),
            (1, Instruction::Iload1),
            (2, Instruction::Putfield(duke_classfile::CpIndex(42))),
            (5, Instruction::Return),
        ];
        let pat = analyze_method_patterns(&instrs);
        assert!(pat.is_setter);
        assert!(!pat.is_getter);
    }

    #[test]
    fn test_allocates_memory() {
        let instrs = vec![
            (0, Instruction::New(duke_classfile::CpIndex(42))),
            (3, Instruction::Return),
        ];
        let pat = analyze_method_patterns(&instrs);
        assert!(pat.allocates_memory);
    }

    #[test]
    fn test_has_loops() {
        let instrs = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Istore1),
            // Loop header
            (2, Instruction::Iload1),
            (3, Instruction::Ifeq(7)), // branch forward
            // Loop body
            (6, Instruction::Goto(-4)), // branch backward to 2
            (9, Instruction::Return),
        ];
        let pat = analyze_method_patterns(&instrs);
        assert!(pat.has_loops);
    }
}
