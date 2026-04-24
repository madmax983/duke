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
    error::{VerifyError, VerifyResult},
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
) -> VerifyResult<()> {
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
            return Err(VerifyError::StackUnderflow { pc });
        }
        depth -= pops;

        // Overflow check
        depth += pushes;
        if depth > max_stack {
            return Err(VerifyError::StackOverflow {
                pc,
                depth,
                max_stack,
            });
        }

        // Empty-stack-on-return check
        if instr.is_normal_return() && depth != 0 {
            return Err(VerifyError::NonEmptyStackOnReturn { pc, depth });
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
// Category-2 values (long, double) count as 1 slot each here because
// we're doing structural depth tracking, not JVM computational-type checking.
// ---------------------------------------------------------------------------

#[allow(clippy::match_same_arms, clippy::too_many_lines)]
/// Check that any local variable accesses are within `max_locals`.
const fn check_locals(instr: &Instruction, pc: usize, max_locals: usize) -> VerifyResult<()> {
    let idx: Option<usize> = instr.local_index();

    if let Some(i) = idx
        && i >= max_locals
    {
        return Err(VerifyError::LocalOutOfBounds {
            pc,
            index: i,
            max_locals,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::ArrayType;

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_stack_effect_coverage() {
        assert_eq!(Instruction::Nop.stack_effect(), (0, 0));
        assert_eq!(Instruction::Lload0.stack_effect(), (0, 1));
        assert_eq!(Instruction::Daload.stack_effect(), (2, 1));
        assert_eq!(Instruction::Dstore0.stack_effect(), (1, 0));
        assert_eq!(Instruction::Lastore.stack_effect(), (3, 0));
        assert_eq!(Instruction::Pop2.stack_effect(), (2, 0));
        assert_eq!(Instruction::Dup2X2.stack_effect(), (4, 6));
        assert_eq!(Instruction::DupX2.stack_effect(), (3, 4));
        assert_eq!(Instruction::Dup2.stack_effect(), (2, 4));
        assert_eq!(Instruction::Istore0.stack_effect(), (1, 0));
        assert_eq!(Instruction::Istore1.stack_effect(), (1, 0));
        assert_eq!(Instruction::Istore2.stack_effect(), (1, 0));
        assert_eq!(Instruction::Istore3.stack_effect(), (1, 0));
        assert_eq!(Instruction::IstoreW(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::Swap.stack_effect(), (2, 2));
        assert_eq!(Instruction::Iadd.stack_effect(), (2, 1));
        assert_eq!(Instruction::Dneg.stack_effect(), (1, 1));
        assert_eq!(
            (Instruction::Iinc { index: 0, value: 1 }).stack_effect(),
            (0, 0)
        );
        assert_eq!(Instruction::I2l.stack_effect(), (1, 1));
        assert_eq!(Instruction::Lcmp.stack_effect(), (2, 1));
        assert_eq!(Instruction::Ifeq(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::IfIcmpne(0).stack_effect(), (2, 0));
        assert_eq!(Instruction::Goto(0).stack_effect(), (0, 0));
        assert_eq!(Instruction::Jsr(0).stack_effect(), (0, 1));
        assert_eq!(Instruction::Ret(0).stack_effect(), (0, 0));
        assert_eq!(Instruction::RetW(0).stack_effect(), (0, 0));
        assert_eq!(Instruction::Return.stack_effect(), (0, 0));
        assert_eq!(Instruction::IloadW(0).stack_effect(), (0, 1));
        assert_eq!(Instruction::Ireturn.stack_effect(), (1, 0));
        assert_eq!(
            Instruction::Getstatic(duke_classfile::CpIndex(1)).stack_effect(),
            (0, 1)
        );
        assert_eq!(
            Instruction::Putstatic(duke_classfile::CpIndex(1)).stack_effect(),
            (1, 0)
        );
        assert_eq!(
            Instruction::Getfield(duke_classfile::CpIndex(1)).stack_effect(),
            (1, 1)
        );
        assert_eq!(
            Instruction::Putfield(duke_classfile::CpIndex(1)).stack_effect(),
            (2, 0)
        );
        assert_eq!(
            Instruction::Invokevirtual(duke_classfile::CpIndex(1)).stack_effect(),
            (1, 0)
        );
        assert_eq!(
            Instruction::Invokestatic(duke_classfile::CpIndex(1)).stack_effect(),
            (0, 0)
        );
        assert_eq!(
            Instruction::New(duke_classfile::CpIndex(1)).stack_effect(),
            (0, 1)
        );
        assert_eq!(Instruction::Newarray(ArrayType::Int).stack_effect(), (1, 1));
        assert_eq!(
            Instruction::Multianewarray {
                index: duke_classfile::CpIndex(1),
                dimensions: 2
            }
            .stack_effect(),
            (2, 1)
        );
        assert_eq!(Instruction::Arraylength.stack_effect(), (1, 1));
        assert_eq!(Instruction::Athrow.stack_effect(), (1, 0));
        assert_eq!(
            Instruction::Checkcast(duke_classfile::CpIndex(1)).stack_effect(),
            (1, 1)
        );
        assert_eq!(Instruction::Monitorenter.stack_effect(), (1, 0));
        assert_eq!(Instruction::DupX1.stack_effect(), (2, 3));
        assert_eq!(Instruction::Dup2X1.stack_effect(), (3, 5));
        assert_eq!(Instruction::Dup.stack_effect(), (1, 2));
        assert_eq!(Instruction::DupX2.stack_effect(), (3, 4));
        assert_eq!(Instruction::Dup2.stack_effect(), (2, 4));
        assert_eq!(Instruction::Bipush(0).stack_effect(), (0, 1));
        assert_eq!(Instruction::Sipush(0).stack_effect(), (0, 1));
        assert_eq!(Instruction::Ldc(1).stack_effect(), (0, 1));
        assert_eq!(
            Instruction::LdcW(duke_classfile::CpIndex(1)).stack_effect(),
            (0, 1)
        );
        assert_eq!(
            Instruction::Ldc2W(duke_classfile::CpIndex(1)).stack_effect(),
            (0, 1)
        );
        assert_eq!(Instruction::Iload(0).stack_effect(), (0, 1));
        assert_eq!(Instruction::Lload(0).stack_effect(), (0, 1));
        assert_eq!(Instruction::Fload(0).stack_effect(), (0, 1));
        assert_eq!(Instruction::Dload(0).stack_effect(), (0, 1));
        assert_eq!(Instruction::Aload(0).stack_effect(), (0, 1));
        assert_eq!(Instruction::Iload1.stack_effect(), (0, 1));
        assert_eq!(Instruction::Iload2.stack_effect(), (0, 1));
        assert_eq!(Instruction::Iload3.stack_effect(), (0, 1));
        assert_eq!(Instruction::Lload1.stack_effect(), (0, 1));
        assert_eq!(Instruction::Lload2.stack_effect(), (0, 1));
        assert_eq!(Instruction::Lload3.stack_effect(), (0, 1));
        assert_eq!(Instruction::Fload0.stack_effect(), (0, 1));
        assert_eq!(Instruction::Fload1.stack_effect(), (0, 1));
        assert_eq!(Instruction::Fload2.stack_effect(), (0, 1));
        assert_eq!(Instruction::Fload3.stack_effect(), (0, 1));
        assert_eq!(Instruction::Dload0.stack_effect(), (0, 1));
        assert_eq!(Instruction::Dload1.stack_effect(), (0, 1));
        assert_eq!(Instruction::Dload2.stack_effect(), (0, 1));
        assert_eq!(Instruction::Dload3.stack_effect(), (0, 1));
        assert_eq!(Instruction::Aload1.stack_effect(), (0, 1));
        assert_eq!(Instruction::Aload2.stack_effect(), (0, 1));
        assert_eq!(Instruction::Aload3.stack_effect(), (0, 1));
        assert_eq!(Instruction::LloadW(0).stack_effect(), (0, 1));
        assert_eq!(Instruction::FloadW(0).stack_effect(), (0, 1));
        assert_eq!(Instruction::DloadW(0).stack_effect(), (0, 1));
        assert_eq!(Instruction::AloadW(0).stack_effect(), (0, 1));
        assert_eq!(Instruction::Iaload.stack_effect(), (2, 1));
        assert_eq!(Instruction::Laload.stack_effect(), (2, 1));
        assert_eq!(Instruction::Faload.stack_effect(), (2, 1));
        assert_eq!(Instruction::Aaload.stack_effect(), (2, 1));
        assert_eq!(Instruction::Baload.stack_effect(), (2, 1));
        assert_eq!(Instruction::Caload.stack_effect(), (2, 1));
        assert_eq!(Instruction::Saload.stack_effect(), (2, 1));
        assert_eq!(Instruction::Istore(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::Lstore(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::Fstore(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::Dstore(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::Astore(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::Istore1.stack_effect(), (1, 0));
        assert_eq!(Instruction::Istore2.stack_effect(), (1, 0));
        assert_eq!(Instruction::Istore3.stack_effect(), (1, 0));
        assert_eq!(Instruction::Lstore0.stack_effect(), (1, 0));
        assert_eq!(Instruction::Lstore1.stack_effect(), (1, 0));
        assert_eq!(Instruction::Lstore2.stack_effect(), (1, 0));
        assert_eq!(Instruction::Lstore3.stack_effect(), (1, 0));
        assert_eq!(Instruction::Fstore0.stack_effect(), (1, 0));
        assert_eq!(Instruction::Fstore1.stack_effect(), (1, 0));
        assert_eq!(Instruction::Fstore2.stack_effect(), (1, 0));
        assert_eq!(Instruction::Fstore3.stack_effect(), (1, 0));
        assert_eq!(Instruction::Dstore1.stack_effect(), (1, 0));
        assert_eq!(Instruction::Dstore2.stack_effect(), (1, 0));
        assert_eq!(Instruction::Dstore3.stack_effect(), (1, 0));
        assert_eq!(Instruction::Astore0.stack_effect(), (1, 0));
        assert_eq!(Instruction::Astore1.stack_effect(), (1, 0));
        assert_eq!(Instruction::Astore2.stack_effect(), (1, 0));
        assert_eq!(Instruction::Astore3.stack_effect(), (1, 0));
        assert_eq!(Instruction::LstoreW(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::FstoreW(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::AstoreW(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::Iastore.stack_effect(), (3, 0));
        assert_eq!(Instruction::Fastore.stack_effect(), (3, 0));
        assert_eq!(Instruction::Dastore.stack_effect(), (3, 0));
        assert_eq!(Instruction::Aastore.stack_effect(), (3, 0));
        assert_eq!(Instruction::Bastore.stack_effect(), (3, 0));
        assert_eq!(Instruction::Castore.stack_effect(), (3, 0));
        assert_eq!(Instruction::Sastore.stack_effect(), (3, 0));
        assert_eq!(Instruction::Pop.stack_effect(), (1, 0));
        assert_eq!(Instruction::Ladd.stack_effect(), (2, 1));
        assert_eq!(Instruction::Fadd.stack_effect(), (2, 1));
        assert_eq!(Instruction::Dadd.stack_effect(), (2, 1));
        assert_eq!(Instruction::Isub.stack_effect(), (2, 1));
        assert_eq!(Instruction::Lsub.stack_effect(), (2, 1));
        assert_eq!(Instruction::Fsub.stack_effect(), (2, 1));
        assert_eq!(Instruction::Dsub.stack_effect(), (2, 1));
        assert_eq!(Instruction::Lmul.stack_effect(), (2, 1));
        assert_eq!(Instruction::Fmul.stack_effect(), (2, 1));
        assert_eq!(Instruction::Dmul.stack_effect(), (2, 1));
        assert_eq!(Instruction::Idiv.stack_effect(), (2, 1));
        assert_eq!(Instruction::Ldiv.stack_effect(), (2, 1));
        assert_eq!(Instruction::Fdiv.stack_effect(), (2, 1));
        assert_eq!(Instruction::Ddiv.stack_effect(), (2, 1));
        assert_eq!(Instruction::Irem.stack_effect(), (2, 1));
        assert_eq!(Instruction::Lrem.stack_effect(), (2, 1));
        assert_eq!(Instruction::Frem.stack_effect(), (2, 1));
        assert_eq!(Instruction::Drem.stack_effect(), (2, 1));
        assert_eq!(Instruction::Ishl.stack_effect(), (2, 1));
        assert_eq!(Instruction::Lshl.stack_effect(), (2, 1));
        assert_eq!(Instruction::Ishr.stack_effect(), (2, 1));
        assert_eq!(Instruction::Lshr.stack_effect(), (2, 1));
        assert_eq!(Instruction::Iushr.stack_effect(), (2, 1));
        assert_eq!(Instruction::Lushr.stack_effect(), (2, 1));
        assert_eq!(Instruction::Iand.stack_effect(), (2, 1));
        assert_eq!(Instruction::Land.stack_effect(), (2, 1));
        assert_eq!(Instruction::Ior.stack_effect(), (2, 1));
        assert_eq!(Instruction::Lor.stack_effect(), (2, 1));
        assert_eq!(Instruction::Ixor.stack_effect(), (2, 1));
        assert_eq!(Instruction::Lxor.stack_effect(), (2, 1));
        assert_eq!(Instruction::Ineg.stack_effect(), (1, 1));
        assert_eq!(Instruction::Lneg.stack_effect(), (1, 1));
        assert_eq!(Instruction::Fneg.stack_effect(), (1, 1));
        assert_eq!(
            (Instruction::IincW { index: 0, value: 1 }).stack_effect(),
            (0, 0)
        );
        assert_eq!(Instruction::I2f.stack_effect(), (1, 1));
        assert_eq!(Instruction::I2d.stack_effect(), (1, 1));
        assert_eq!(Instruction::L2i.stack_effect(), (1, 1));
        assert_eq!(Instruction::L2f.stack_effect(), (1, 1));
        assert_eq!(Instruction::L2d.stack_effect(), (1, 1));
        assert_eq!(Instruction::F2i.stack_effect(), (1, 1));
        assert_eq!(Instruction::F2l.stack_effect(), (1, 1));
        assert_eq!(Instruction::F2d.stack_effect(), (1, 1));
        assert_eq!(Instruction::D2i.stack_effect(), (1, 1));
        assert_eq!(Instruction::D2l.stack_effect(), (1, 1));
        assert_eq!(Instruction::D2f.stack_effect(), (1, 1));
        assert_eq!(Instruction::I2b.stack_effect(), (1, 1));
        assert_eq!(Instruction::I2c.stack_effect(), (1, 1));
        assert_eq!(Instruction::I2s.stack_effect(), (1, 1));
        assert_eq!(Instruction::Fcmpl.stack_effect(), (2, 1));
        assert_eq!(Instruction::Fcmpg.stack_effect(), (2, 1));
        assert_eq!(Instruction::Dcmpl.stack_effect(), (2, 1));
        assert_eq!(Instruction::Dcmpg.stack_effect(), (2, 1));
        assert_eq!(Instruction::Ifne(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::Iflt(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::Ifge(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::Ifgt(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::Ifle(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::Ifnull(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::Ifnonnull(0).stack_effect(), (1, 0));
        assert_eq!(Instruction::IfIcmpeq(0).stack_effect(), (2, 0));
        assert_eq!(Instruction::IfIcmplt(0).stack_effect(), (2, 0));
        assert_eq!(Instruction::IfIcmpge(0).stack_effect(), (2, 0));
        assert_eq!(Instruction::IfIcmpgt(0).stack_effect(), (2, 0));
        assert_eq!(Instruction::IfIcmple(0).stack_effect(), (2, 0));
        assert_eq!(Instruction::IfAcmpeq(0).stack_effect(), (2, 0));
        assert_eq!(Instruction::IfAcmpne(0).stack_effect(), (2, 0));
        assert_eq!(Instruction::GotoW(0).stack_effect(), (0, 0));
        assert_eq!(Instruction::JsrW(0).stack_effect(), (0, 1));
        assert_eq!(Instruction::Lreturn.stack_effect(), (1, 0));
        assert_eq!(Instruction::Freturn.stack_effect(), (1, 0));
        assert_eq!(Instruction::Dreturn.stack_effect(), (1, 0));
        assert_eq!(Instruction::Areturn.stack_effect(), (1, 0));
        assert_eq!(
            Instruction::Invokespecial(duke_classfile::CpIndex(1)).stack_effect(),
            (1, 0)
        );
        assert_eq!(Instruction::Monitorexit.stack_effect(), (1, 0));

        assert_eq!(
            Instruction::Tableswitch {
                default: 0,
                low: 0,
                high: 0,
                offsets: vec![],
            }
            .stack_effect(),
            (1, 0)
        );
        assert_eq!(
            Instruction::Lookupswitch {
                default: 0,
                pairs: vec![],
            }
            .stack_effect(),
            (1, 0)
        );
        assert_eq!(
            Instruction::Invokeinterface {
                index: duke_classfile::CpIndex(1),
                count: 1,
            }
            .stack_effect(),
            (1, 0)
        );
        assert_eq!(
            Instruction::Invokedynamic(duke_classfile::CpIndex(1)).stack_effect(),
            (0, 0)
        );
        assert_eq!(
            Instruction::Instanceof(duke_classfile::CpIndex(1)).stack_effect(),
            (1, 1)
        );
        assert_eq!(
            Instruction::Anewarray(duke_classfile::CpIndex(1)).stack_effect(),
            (1, 1)
        );
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

    #[test]
    fn test_stack_effect_math() {
        assert_eq!(Instruction::Imul.stack_effect(), (2, 1));
        assert_eq!(Instruction::DupX2.stack_effect(), (3, 4));
        assert_eq!(Instruction::Dup2X2.stack_effect(), (4, 6));
        assert_eq!(Instruction::Swap.stack_effect(), (2, 2));
    }

    #[test]
    fn test_check_locals_ret() {
        let instructions = vec![(0, Instruction::Ret(5)), (2, Instruction::Return)];
        let res = verify(&instructions, 1, 5); // max locals is 5, index 5 is out of bounds
        assert!(matches!(
            res,
            Err(VerifyError::LocalOutOfBounds { index: 5, .. })
        ));

        let instructions_w = vec![(0, Instruction::RetW(10)), (3, Instruction::Return)];
        let res_w = verify(&instructions_w, 1, 10);
        assert!(matches!(
            res_w,
            Err(VerifyError::LocalOutOfBounds { index: 10, .. })
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
            Err(VerifyError::LocalOutOfBounds { index: 5, .. })
        ));

        let instructions_store = vec![(0, Instruction::IstoreW(10)), (3, Instruction::Return)];
        let res = verify(&instructions_store, 1, 10);
        assert!(matches!(
            res,
            Err(VerifyError::LocalOutOfBounds { index: 10, .. })
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
            Err(VerifyError::LocalOutOfBounds { index: 5, .. })
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
            Err(VerifyError::LocalOutOfBounds { index: 10, .. })
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
    fn should_return_error_when_local_index_is_out_of_bounds() {
        let instr = Instruction::Iload(5);
        let result = check_locals(&instr, 0, 4);
        assert!(matches!(
            result,
            Err(VerifyError::LocalOutOfBounds {
                pc: 0,
                index: 5,
                max_locals: 4
            })
        ));

        let instr = Instruction::Dload(5);
        let result = check_locals(&instr, 0, 4);
        assert!(matches!(
            result,
            Err(VerifyError::LocalOutOfBounds {
                pc: 0,
                index: 5,
                max_locals: 4
            })
        ));

        let instr = Instruction::IincW { index: 5, value: 1 };
        let result = check_locals(&instr, 0, 4);
        assert!(matches!(
            result,
            Err(VerifyError::LocalOutOfBounds {
                pc: 0,
                index: 5,
                max_locals: 4
            })
        ));

        let instr = Instruction::Dload(5);
        let result = check_locals(&instr, 0, 4);
        assert!(matches!(
            result,
            Err(VerifyError::LocalOutOfBounds {
                pc: 0,
                index: 5,
                max_locals: 4
            })
        ));

        let instr = Instruction::DloadW(5);
        let result = check_locals(&instr, 0, 4);
        assert!(matches!(
            result,
            Err(VerifyError::LocalOutOfBounds {
                pc: 0,
                index: 5,
                max_locals: 4
            })
        ));

        let instr = Instruction::IincW { index: 5, value: 1 };
        let result = check_locals(&instr, 0, 4);
        assert!(matches!(
            result,
            Err(VerifyError::LocalOutOfBounds {
                pc: 0,
                index: 5,
                max_locals: 4
            })
        ));
    }

    #[test]
    fn test_verifier_short_form_loads_oob() {
        assert!(matches!(
            check_locals(&Instruction::Iload0, 0, 0),
            Err(VerifyError::LocalOutOfBounds {
                pc: 0,
                index: 0,
                max_locals: 0
            })
        ));
        assert!(matches!(
            check_locals(&Instruction::Dload0, 0, 0),
            Err(VerifyError::LocalOutOfBounds {
                pc: 0,
                index: 0,
                max_locals: 0
            })
        ));
    }
}
