//! Structural bytecode verifier (JVM §4.10 subset).
//!
//! Performs a forward linear scan over decoded instructions and checks:
//!
//! 1. **Stack depth** never exceeds `max_stack`
//! 2. **Stack is never underflowed** (popping from depth 0)
//! 3. **Local variable indices** are within `max_locals`
//! 4. **Stack is empty** at every return instruction
//!
//! This is the structural half of JVM verification. Full type-checking
//! (abstract interpretation with type states at merge points) is Phase 2b.

use crate::{
    error::{Result, VerifyError},
    instruction::Instruction,
};

/// Verify a decoded instruction stream structurally.
///
/// Ensures the basic integrity of a parsed method's instructions before
/// interpretation. This prevents stack underflows or invalid variable access
/// during execution.
///
/// # Parameters
///
/// - `instructions`: output of [`crate::decoder::decode`]
/// - `max_stack`: from the `Code` attribute
/// - `max_locals`: from the `Code` attribute
///
/// # Errors
///
/// Returns the first [`VerifyError`] encountered.
///
/// # Examples
///
/// ```
/// use duke_bytecode::{verify, Instruction};
///
/// let code = [(0, Instruction::Iconst1), (1, Instruction::Ireturn)];
/// // Valid: max_stack=1, max_locals=0
/// assert!(verify(&code, 1, 0).is_ok());
/// ```
///
/// ```
/// use duke_bytecode::{verify, Instruction};
///
/// let bad_code = [(0, Instruction::Iload0), (1, Instruction::Ireturn)];
/// // Invalid: max_locals=0 means iload_0 will read out of bounds
/// assert!(verify(&bad_code, 1, 0).is_err());
/// ```
pub fn verify(
    instructions: &[(usize, Instruction)],
    max_stack: u16,
    max_locals: u16,
) -> Result<()> {
    let max_stack = max_stack as usize;
    let max_locals = max_locals as usize;
    let mut depth: usize = 0;

    for (pc, instr) in instructions {
        let pc = *pc;

        let (pops, pushes) = instr.stack_effect();

        // Check locals before stack — gives a more meaningful error on load/store.
        check_locals(instr, pc, max_locals)?;

        // Underflow check
        if depth < pops {
            return Err(crate::Error::Verify(VerifyError::StackUnderflow { pc }));
        }
        depth -= pops;

        // Overflow check
        depth += pushes;
        if depth > max_stack {
            return Err(crate::Error::Verify(VerifyError::StackOverflow {
                pc,
                depth,
                max_stack,
            }));
        }

        // Empty-stack-on-return check
        if is_return(instr) && depth != 0 {
            return Err(crate::Error::Verify(VerifyError::NonEmptyStackOnReturn {
                pc,
                depth,
            }));
        }

        // athrow consumes the exception reference; stack is conceptually cleared
        // after throw (execution does not continue linearly), so reset depth.
        if matches!(instr, Instruction::Athrow) {
            depth = 0;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Stack-effect table
//
// Returns (pops, pushes) for each instruction.

const fn is_return(instr: &Instruction) -> bool {
    matches!(
        instr,
        Instruction::Return
            | Instruction::Ireturn
            | Instruction::Lreturn
            | Instruction::Freturn
            | Instruction::Dreturn
            | Instruction::Areturn
    )
}

/// Check that any local variable accesses are within `max_locals`.
const fn check_locals(instr: &Instruction, pc: usize, max_locals: usize) -> Result<()> {
    if let Some(i) = instr.accessed_local_index()
        && i >= max_locals
    {
        return Err(crate::Error::Verify(VerifyError::LocalOutOfBounds {
            pc,
            index: i,
            max_locals,
        }));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_check_locals_ret() {
        let instructions = vec![(0, Instruction::Ret(5)), (2, Instruction::Return)];
        let res = verify(&instructions, 1, 5); // max locals is 5, index 5 is out of bounds
        assert!(matches!(
            res,
            Err(crate::Error::Verify(VerifyError::LocalOutOfBounds {
                index: 5,
                ..
            }))
        ));

        let instructions_w = vec![(0, Instruction::RetW(10)), (3, Instruction::Return)];
        let res_w = verify(&instructions_w, 1, 10);
        assert!(matches!(
            res_w,
            Err(crate::Error::Verify(VerifyError::LocalOutOfBounds {
                index: 10,
                ..
            }))
        ));

        // Test valid RetW
        let instructions_w_valid = vec![(0, Instruction::RetW(10)), (3, Instruction::Return)];
        let res_w_valid = verify(&instructions_w_valid, 1, 11);
        assert!(res_w_valid.is_ok());
    }

    #[test]
    fn test_verifier_local_oob() {
        let instructions = vec![(0, Instruction::Iload(5)), (2, Instruction::Return)];
        let res = verify(&instructions, 1, 5); // max locals is 5, index 5 is out of bounds
        assert!(matches!(
            res,
            Err(crate::Error::Verify(VerifyError::LocalOutOfBounds {
                index: 5,
                ..
            }))
        ));

        let instructions_store = vec![(0, Instruction::IstoreW(10)), (3, Instruction::Return)];
        let res = verify(&instructions_store, 1, 10);
        assert!(matches!(
            res,
            Err(crate::Error::Verify(VerifyError::LocalOutOfBounds {
                index: 10,
                ..
            }))
        ));
    }

    #[test]
    fn test_verifier_local_oob_iinc() {
        let instructions = vec![
            (0, Instruction::Iinc { index: 5, value: 1 }),
            (3, Instruction::Return),
        ];
        let res = verify(&instructions, 1, 5);
        assert!(matches!(
            res,
            Err(crate::Error::Verify(VerifyError::LocalOutOfBounds {
                index: 5,
                ..
            }))
        ));

        let instructions_wide = vec![
            (
                0,
                Instruction::IincW {
                    index: 10,
                    value: 1,
                },
            ),
            (4, Instruction::Return),
        ];
        let res = verify(&instructions_wide, 1, 10);
        assert!(matches!(
            res,
            Err(crate::Error::Verify(VerifyError::LocalOutOfBounds {
                index: 10,
                ..
            }))
        ));

        // Test valid IincW
        let instructions_wide_valid = vec![
            (
                0,
                Instruction::IincW {
                    index: 10,
                    value: 1,
                },
            ),
            (4, Instruction::Return),
        ];
        let res_valid = verify(&instructions_wide_valid, 1, 11);
        assert!(res_valid.is_ok());
    }

    #[test]
    fn should_return_error_for_all_local_access_out_of_bounds() {
        let max_locals = 4;
        let oob_index = 5;

        // Exhaustive list of instructions that take an index
        let oob_instructions = vec![
            Instruction::Iload(oob_index),
            Instruction::Lload(oob_index),
            Instruction::Fload(oob_index),
            Instruction::Dload(oob_index),
            Instruction::Aload(oob_index),
            Instruction::Istore(oob_index),
            Instruction::Lstore(oob_index),
            Instruction::Fstore(oob_index),
            Instruction::Dstore(oob_index),
            Instruction::Astore(oob_index),
            Instruction::Ret(oob_index),
            Instruction::Iinc {
                index: oob_index,
                value: 1,
            },
            Instruction::IloadW(u16::from(oob_index)),
            Instruction::LloadW(u16::from(oob_index)),
            Instruction::FloadW(u16::from(oob_index)),
            Instruction::DloadW(u16::from(oob_index)),
            Instruction::AloadW(u16::from(oob_index)),
            Instruction::IstoreW(u16::from(oob_index)),
            Instruction::LstoreW(u16::from(oob_index)),
            Instruction::FstoreW(u16::from(oob_index)),
            Instruction::DstoreW(u16::from(oob_index)),
            Instruction::AstoreW(u16::from(oob_index)),
            Instruction::RetW(u16::from(oob_index)),
            Instruction::IincW {
                index: u16::from(oob_index),
                value: 1,
            },
        ];

        for instr in oob_instructions {
            let result = check_locals(&instr, 0, max_locals);
            assert!(
                matches!(
                    result,
                    Err(crate::Error::Verify(VerifyError::LocalOutOfBounds {
                        pc: 0,
                        index: 5,
                        max_locals: 4
                    }))
                ),
                "Instruction {instr:?} failed to return LocalOutOfBounds error",
            );
        }

        // Exhaustive list of short form instructions that implicitly access locals
        // Max locals = 2, so accessing 2 or 3 is out of bounds
        let max_locals_short = 2;
        let oob_short_instructions = vec![
            (Instruction::Iload2, 2),
            (Instruction::Iload3, 3),
            (Instruction::Lload2, 2),
            (Instruction::Lload3, 3),
            (Instruction::Fload2, 2),
            (Instruction::Fload3, 3),
            (Instruction::Dload2, 2),
            (Instruction::Dload3, 3),
            (Instruction::Aload2, 2),
            (Instruction::Aload3, 3),
            (Instruction::Istore2, 2),
            (Instruction::Istore3, 3),
            (Instruction::Lstore2, 2),
            (Instruction::Lstore3, 3),
            (Instruction::Fstore2, 2),
            (Instruction::Fstore3, 3),
            (Instruction::Dstore2, 2),
            (Instruction::Dstore3, 3),
            (Instruction::Astore2, 2),
            (Instruction::Astore3, 3),
        ];

        for (instr, expected_index) in oob_short_instructions {
            let result = check_locals(&instr, 0, max_locals_short);
            assert!(
                matches!(
                    result,
                    Err(crate::Error::Verify(VerifyError::LocalOutOfBounds {
                        pc: 0,
                        index: idx,
                        max_locals: 2
                    })) if idx == expected_index
                ),
                "Instruction {instr:?} failed to return LocalOutOfBounds error",
            );
        }
    }
    #[test]
    fn test_verifier_athrow_resets_stack() {
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Iconst0),
            (2, Instruction::Athrow),
            (3, Instruction::Return), // stack is 0 here after athrow
        ];
        // Athrow pops 1 but then stack is reset to 0
        let res = verify(&instructions, 2, 0);
        assert!(res.is_ok());
    }
}
