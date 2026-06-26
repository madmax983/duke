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
/// Returns `Err` with the first `VerifyError` encountered if the code array fails structural
/// verification (e.g., jumping to an invalid offset or exceeding `max_stack`).
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

        let (pops, pushes) = stack_effect(instr);

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
// Category-2 values (long, double) count as 1 slot each here because
// we're doing structural depth tracking, not JVM computational-type checking.
// ---------------------------------------------------------------------------

#[allow(clippy::match_same_arms, clippy::too_many_lines)]
const fn stack_effect(instr: &Instruction) -> (usize, usize) {
    match instr {
        // Constants — push 1
        Instruction::Nop => (0, 0),
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
        | Instruction::Ldc2W(_) => (0, 1),

        // Loads — push 1
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
        | Instruction::IloadW(_)
        | Instruction::LloadW(_)
        | Instruction::FloadW(_)
        | Instruction::DloadW(_)
        | Instruction::AloadW(_) => (0, 1),

        // Array loads — pop arrayref + index, push value
        Instruction::Iaload
        | Instruction::Laload
        | Instruction::Faload
        | Instruction::Daload
        | Instruction::Aaload
        | Instruction::Baload
        | Instruction::Caload
        | Instruction::Saload => (2, 1),

        // Stores — pop 1
        Instruction::Istore(_)
        | Instruction::Lstore(_)
        | Instruction::Fstore(_)
        | Instruction::Dstore(_)
        | Instruction::Astore(_)
        | Instruction::Istore0
        | Instruction::Istore1
        | Instruction::Istore2
        | Instruction::Istore3
        | Instruction::Lstore0
        | Instruction::Lstore1
        | Instruction::Lstore2
        | Instruction::Lstore3
        | Instruction::Fstore0
        | Instruction::Fstore1
        | Instruction::Fstore2
        | Instruction::Fstore3
        | Instruction::Dstore0
        | Instruction::Dstore1
        | Instruction::Dstore2
        | Instruction::Dstore3
        | Instruction::Astore0
        | Instruction::Astore1
        | Instruction::Astore2
        | Instruction::Astore3
        | Instruction::IstoreW(_)
        | Instruction::LstoreW(_)
        | Instruction::FstoreW(_)
        | Instruction::DstoreW(_)
        | Instruction::AstoreW(_) => (1, 0),

        // Array stores — pop arrayref + index + value
        Instruction::Iastore
        | Instruction::Lastore
        | Instruction::Fastore
        | Instruction::Dastore
        | Instruction::Aastore
        | Instruction::Bastore
        | Instruction::Castore
        | Instruction::Sastore => (3, 0),

        // Stack ops
        Instruction::Pop => (1, 0),
        Instruction::Pop2 => (2, 0),
        Instruction::Dup => (1, 2),
        Instruction::DupX1 => (2, 3),
        Instruction::DupX2 => (3, 4),
        Instruction::Dup2 => (2, 4),
        Instruction::Dup2X1 => (3, 5),
        Instruction::Dup2X2 => (4, 6),
        Instruction::Swap => (2, 2),

        // Binary arithmetic — pop 2, push 1
        Instruction::Iadd
        | Instruction::Ladd
        | Instruction::Fadd
        | Instruction::Dadd
        | Instruction::Isub
        | Instruction::Lsub
        | Instruction::Fsub
        | Instruction::Dsub
        | Instruction::Imul
        | Instruction::Lmul
        | Instruction::Fmul
        | Instruction::Dmul
        | Instruction::Idiv
        | Instruction::Ldiv
        | Instruction::Fdiv
        | Instruction::Ddiv
        | Instruction::Irem
        | Instruction::Lrem
        | Instruction::Frem
        | Instruction::Drem
        | Instruction::Ishl
        | Instruction::Lshl
        | Instruction::Ishr
        | Instruction::Lshr
        | Instruction::Iushr
        | Instruction::Lushr
        | Instruction::Iand
        | Instruction::Land
        | Instruction::Ior
        | Instruction::Lor
        | Instruction::Ixor
        | Instruction::Lxor => (2, 1),

        // Unary arithmetic — pop 1, push 1
        Instruction::Ineg | Instruction::Lneg | Instruction::Fneg | Instruction::Dneg => (1, 1),

        // iinc — operates on local, no stack change
        Instruction::Iinc { .. } | Instruction::IincW { .. } => (0, 0),

        // Conversions — pop 1, push 1
        Instruction::I2l
        | Instruction::I2f
        | Instruction::I2d
        | Instruction::L2i
        | Instruction::L2f
        | Instruction::L2d
        | Instruction::F2i
        | Instruction::F2l
        | Instruction::F2d
        | Instruction::D2i
        | Instruction::D2l
        | Instruction::D2f
        | Instruction::I2b
        | Instruction::I2c
        | Instruction::I2s => (1, 1),

        // Compare — pop 2, push 1 (int result: -1/0/1)
        Instruction::Lcmp
        | Instruction::Fcmpl
        | Instruction::Fcmpg
        | Instruction::Dcmpl
        | Instruction::Dcmpg => (2, 1),

        // Conditional branches — pop 1 (for if*) or 2 (for if_icmp* / if_acmp*)
        Instruction::Ifeq(_)
        | Instruction::Ifne(_)
        | Instruction::Iflt(_)
        | Instruction::Ifge(_)
        | Instruction::Ifgt(_)
        | Instruction::Ifle(_)
        | Instruction::Ifnull(_)
        | Instruction::Ifnonnull(_) => (1, 0),

        Instruction::IfIcmpeq(_)
        | Instruction::IfIcmpne(_)
        | Instruction::IfIcmplt(_)
        | Instruction::IfIcmpge(_)
        | Instruction::IfIcmpgt(_)
        | Instruction::IfIcmple(_)
        | Instruction::IfAcmpeq(_)
        | Instruction::IfAcmpne(_) => (2, 0),

        // Unconditional branches — no stack change
        Instruction::Goto(_) | Instruction::GotoW(_) => (0, 0),

        // jsr pushes return address; ret pops nothing
        Instruction::Jsr(_) | Instruction::JsrW(_) => (0, 1),
        Instruction::Ret(_) | Instruction::RetW(_) => (0, 0),

        // switch — pop 1
        Instruction::Tableswitch { .. } | Instruction::Lookupswitch { .. } => (1, 0),

        // Returns — pop 0 or 1 (void vs value return)
        Instruction::Return => (0, 0),
        Instruction::Ireturn
        | Instruction::Lreturn
        | Instruction::Freturn
        | Instruction::Dreturn
        | Instruction::Areturn => (1, 0),

        // Field access
        Instruction::Getstatic(_) => (0, 1),
        Instruction::Putstatic(_) => (1, 0),
        Instruction::Getfield(_) => (1, 1),
        Instruction::Putfield(_) => (2, 0),

        // Method invocations — conservative: just track the hidden receiver pop
        // Proper argument counting requires descriptor resolution; tracked in Phase 3+
        Instruction::Invokevirtual(_) | Instruction::Invokespecial(_) => (1, 0),
        Instruction::Invokestatic(_) => (0, 0),
        Instruction::Invokeinterface { .. } => (1, 0),
        Instruction::Invokedynamic(_) => (0, 0),

        // Object creation — push objectref
        Instruction::New(_) => (0, 1),
        Instruction::Newarray(_) | Instruction::Anewarray(_) => (1, 1),
        Instruction::Multianewarray { dimensions, .. } => (*dimensions as usize, 1),
        Instruction::Arraylength => (1, 1),

        // Athrow — handled specially (resets depth) but structurally pops 1
        Instruction::Athrow => (1, 0),

        // Checkcast: pops objectref, pushes same (or throws)
        Instruction::Checkcast(_) => (1, 1),
        Instruction::Instanceof(_) => (1, 1),

        Instruction::Monitorenter | Instruction::Monitorexit => (1, 0),
    }
}

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
    let idx: Option<usize> = match instr {
        Instruction::Iload(i)
        | Instruction::Lload(i)
        | Instruction::Fload(i)
        | Instruction::Dload(i)
        | Instruction::Aload(i)
        | Instruction::Istore(i)
        | Instruction::Lstore(i)
        | Instruction::Fstore(i)
        | Instruction::Dstore(i)
        | Instruction::Astore(i)
        | Instruction::Ret(i) => Some(*i as usize),

        Instruction::Iinc { index, .. } => Some(*index as usize),

        Instruction::IloadW(i)
        | Instruction::LloadW(i)
        | Instruction::FloadW(i)
        | Instruction::DloadW(i)
        | Instruction::AloadW(i)
        | Instruction::IstoreW(i)
        | Instruction::LstoreW(i)
        | Instruction::FstoreW(i)
        | Instruction::DstoreW(i)
        | Instruction::AstoreW(i)
        | Instruction::RetW(i) => Some(*i as usize),

        Instruction::IincW { index, .. } => Some(*index as usize),

        // Short-form loads/stores use fixed indices 0-3, always valid if max_locals >= 1
        Instruction::Iload0
        | Instruction::Lload0
        | Instruction::Fload0
        | Instruction::Dload0
        | Instruction::Aload0
        | Instruction::Istore0
        | Instruction::Lstore0
        | Instruction::Fstore0
        | Instruction::Dstore0
        | Instruction::Astore0 => Some(0),

        Instruction::Iload1
        | Instruction::Lload1
        | Instruction::Fload1
        | Instruction::Dload1
        | Instruction::Aload1
        | Instruction::Istore1
        | Instruction::Lstore1
        | Instruction::Fstore1
        | Instruction::Dstore1
        | Instruction::Astore1 => Some(1),

        Instruction::Iload2
        | Instruction::Lload2
        | Instruction::Fload2
        | Instruction::Dload2
        | Instruction::Aload2
        | Instruction::Istore2
        | Instruction::Lstore2
        | Instruction::Fstore2
        | Instruction::Dstore2
        | Instruction::Astore2 => Some(2),

        Instruction::Iload3
        | Instruction::Lload3
        | Instruction::Fload3
        | Instruction::Dload3
        | Instruction::Aload3
        | Instruction::Istore3
        | Instruction::Lstore3
        | Instruction::Fstore3
        | Instruction::Dstore3
        | Instruction::Astore3 => Some(3),

        _ => None,
    };

    if let Some(i) = idx
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
    use crate::instruction::ArrayType;

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_stack_effect_coverage() {
        assert_eq!(stack_effect(&Instruction::Nop), (0, 0));
        assert_eq!(stack_effect(&Instruction::Lload0), (0, 1));
        assert_eq!(stack_effect(&Instruction::Daload), (2, 1));
        assert_eq!(stack_effect(&Instruction::Dstore0), (1, 0));
        assert_eq!(stack_effect(&Instruction::Lastore), (3, 0));
        assert_eq!(stack_effect(&Instruction::Pop2), (2, 0));
        assert_eq!(stack_effect(&Instruction::Dup2X2), (4, 6));
        assert_eq!(stack_effect(&Instruction::DupX2), (3, 4));
        assert_eq!(stack_effect(&Instruction::Dup2), (2, 4));
        assert_eq!(stack_effect(&Instruction::Istore0), (1, 0));
        assert_eq!(stack_effect(&Instruction::Istore1), (1, 0));
        assert_eq!(stack_effect(&Instruction::Istore2), (1, 0));
        assert_eq!(stack_effect(&Instruction::Istore3), (1, 0));
        assert_eq!(stack_effect(&Instruction::IstoreW(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::Swap), (2, 2));
        assert_eq!(stack_effect(&Instruction::Iadd), (2, 1));
        assert_eq!(stack_effect(&Instruction::Dneg), (1, 1));
        assert_eq!(
            stack_effect(&Instruction::Iinc { index: 0, value: 1 }),
            (0, 0)
        );
        assert_eq!(stack_effect(&Instruction::I2l), (1, 1));
        assert_eq!(stack_effect(&Instruction::Lcmp), (2, 1));
        assert_eq!(stack_effect(&Instruction::Ifeq(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::IfIcmpne(0)), (2, 0));
        assert_eq!(stack_effect(&Instruction::Goto(0)), (0, 0));
        assert_eq!(stack_effect(&Instruction::Jsr(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::Ret(0)), (0, 0));
        assert_eq!(stack_effect(&Instruction::RetW(0)), (0, 0));
        assert_eq!(stack_effect(&Instruction::Return), (0, 0));
        assert_eq!(stack_effect(&Instruction::IloadW(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::Ireturn), (1, 0));
        assert_eq!(
            stack_effect(&Instruction::Getstatic(duke_classfile::CpIndex(1))),
            (0, 1)
        );
        assert_eq!(
            stack_effect(&Instruction::Putstatic(duke_classfile::CpIndex(1))),
            (1, 0)
        );
        assert_eq!(
            stack_effect(&Instruction::Getfield(duke_classfile::CpIndex(1))),
            (1, 1)
        );
        assert_eq!(
            stack_effect(&Instruction::Putfield(duke_classfile::CpIndex(1))),
            (2, 0)
        );
        assert_eq!(
            stack_effect(&Instruction::Invokevirtual(duke_classfile::CpIndex(1))),
            (1, 0)
        );
        assert_eq!(
            stack_effect(&Instruction::Invokestatic(duke_classfile::CpIndex(1))),
            (0, 0)
        );
        assert_eq!(
            stack_effect(&Instruction::New(duke_classfile::CpIndex(1))),
            (0, 1)
        );
        assert_eq!(stack_effect(&Instruction::Newarray(ArrayType::Int)), (1, 1));
        assert_eq!(
            stack_effect(&Instruction::Multianewarray {
                index: duke_classfile::CpIndex(1),
                dimensions: 2
            }),
            (2, 1)
        );
        assert_eq!(stack_effect(&Instruction::Arraylength), (1, 1));
        assert_eq!(stack_effect(&Instruction::Athrow), (1, 0));
        assert_eq!(
            stack_effect(&Instruction::Checkcast(duke_classfile::CpIndex(1))),
            (1, 1)
        );
        assert_eq!(stack_effect(&Instruction::Monitorenter), (1, 0));
        assert_eq!(stack_effect(&Instruction::DupX1), (2, 3));
        assert_eq!(stack_effect(&Instruction::Dup2X1), (3, 5));
        assert_eq!(stack_effect(&Instruction::Dup), (1, 2));
        assert_eq!(stack_effect(&Instruction::DupX2), (3, 4));
        assert_eq!(stack_effect(&Instruction::Dup2), (2, 4));
        assert_eq!(stack_effect(&Instruction::Bipush(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::Sipush(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::Ldc(1)), (0, 1));
        assert_eq!(
            stack_effect(&Instruction::LdcW(duke_classfile::CpIndex(1))),
            (0, 1)
        );
        assert_eq!(
            stack_effect(&Instruction::Ldc2W(duke_classfile::CpIndex(1))),
            (0, 1)
        );
        assert_eq!(stack_effect(&Instruction::Iload(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::Lload(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::Fload(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::Dload(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::Aload(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::Iload1), (0, 1));
        assert_eq!(stack_effect(&Instruction::Iload2), (0, 1));
        assert_eq!(stack_effect(&Instruction::Iload3), (0, 1));
        assert_eq!(stack_effect(&Instruction::Lload1), (0, 1));
        assert_eq!(stack_effect(&Instruction::Lload2), (0, 1));
        assert_eq!(stack_effect(&Instruction::Lload3), (0, 1));
        assert_eq!(stack_effect(&Instruction::Fload0), (0, 1));
        assert_eq!(stack_effect(&Instruction::Fload1), (0, 1));
        assert_eq!(stack_effect(&Instruction::Fload2), (0, 1));
        assert_eq!(stack_effect(&Instruction::Fload3), (0, 1));
        assert_eq!(stack_effect(&Instruction::Dload0), (0, 1));
        assert_eq!(stack_effect(&Instruction::Dload1), (0, 1));
        assert_eq!(stack_effect(&Instruction::Dload2), (0, 1));
        assert_eq!(stack_effect(&Instruction::Dload3), (0, 1));
        assert_eq!(stack_effect(&Instruction::Aload1), (0, 1));
        assert_eq!(stack_effect(&Instruction::Aload2), (0, 1));
        assert_eq!(stack_effect(&Instruction::Aload3), (0, 1));
        assert_eq!(stack_effect(&Instruction::LloadW(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::FloadW(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::DloadW(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::AloadW(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::Iaload), (2, 1));
        assert_eq!(stack_effect(&Instruction::Laload), (2, 1));
        assert_eq!(stack_effect(&Instruction::Faload), (2, 1));
        assert_eq!(stack_effect(&Instruction::Aaload), (2, 1));
        assert_eq!(stack_effect(&Instruction::Baload), (2, 1));
        assert_eq!(stack_effect(&Instruction::Caload), (2, 1));
        assert_eq!(stack_effect(&Instruction::Saload), (2, 1));
        assert_eq!(stack_effect(&Instruction::Istore(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::Lstore(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::Fstore(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::Dstore(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::Astore(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::Istore1), (1, 0));
        assert_eq!(stack_effect(&Instruction::Istore2), (1, 0));
        assert_eq!(stack_effect(&Instruction::Istore3), (1, 0));
        assert_eq!(stack_effect(&Instruction::Lstore0), (1, 0));
        assert_eq!(stack_effect(&Instruction::Lstore1), (1, 0));
        assert_eq!(stack_effect(&Instruction::Lstore2), (1, 0));
        assert_eq!(stack_effect(&Instruction::Lstore3), (1, 0));
        assert_eq!(stack_effect(&Instruction::Fstore0), (1, 0));
        assert_eq!(stack_effect(&Instruction::Fstore1), (1, 0));
        assert_eq!(stack_effect(&Instruction::Fstore2), (1, 0));
        assert_eq!(stack_effect(&Instruction::Fstore3), (1, 0));
        assert_eq!(stack_effect(&Instruction::Dstore1), (1, 0));
        assert_eq!(stack_effect(&Instruction::Dstore2), (1, 0));
        assert_eq!(stack_effect(&Instruction::Dstore3), (1, 0));
        assert_eq!(stack_effect(&Instruction::Astore0), (1, 0));
        assert_eq!(stack_effect(&Instruction::Astore1), (1, 0));
        assert_eq!(stack_effect(&Instruction::Astore2), (1, 0));
        assert_eq!(stack_effect(&Instruction::Astore3), (1, 0));
        assert_eq!(stack_effect(&Instruction::LstoreW(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::FstoreW(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::AstoreW(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::Iastore), (3, 0));
        assert_eq!(stack_effect(&Instruction::Fastore), (3, 0));
        assert_eq!(stack_effect(&Instruction::Dastore), (3, 0));
        assert_eq!(stack_effect(&Instruction::Aastore), (3, 0));
        assert_eq!(stack_effect(&Instruction::Bastore), (3, 0));
        assert_eq!(stack_effect(&Instruction::Castore), (3, 0));
        assert_eq!(stack_effect(&Instruction::Sastore), (3, 0));
        assert_eq!(stack_effect(&Instruction::Pop), (1, 0));
        assert_eq!(stack_effect(&Instruction::Ladd), (2, 1));
        assert_eq!(stack_effect(&Instruction::Fadd), (2, 1));
        assert_eq!(stack_effect(&Instruction::Dadd), (2, 1));
        assert_eq!(stack_effect(&Instruction::Isub), (2, 1));
        assert_eq!(stack_effect(&Instruction::Lsub), (2, 1));
        assert_eq!(stack_effect(&Instruction::Fsub), (2, 1));
        assert_eq!(stack_effect(&Instruction::Dsub), (2, 1));
        assert_eq!(stack_effect(&Instruction::Lmul), (2, 1));
        assert_eq!(stack_effect(&Instruction::Fmul), (2, 1));
        assert_eq!(stack_effect(&Instruction::Dmul), (2, 1));
        assert_eq!(stack_effect(&Instruction::Idiv), (2, 1));
        assert_eq!(stack_effect(&Instruction::Ldiv), (2, 1));
        assert_eq!(stack_effect(&Instruction::Fdiv), (2, 1));
        assert_eq!(stack_effect(&Instruction::Ddiv), (2, 1));
        assert_eq!(stack_effect(&Instruction::Irem), (2, 1));
        assert_eq!(stack_effect(&Instruction::Lrem), (2, 1));
        assert_eq!(stack_effect(&Instruction::Frem), (2, 1));
        assert_eq!(stack_effect(&Instruction::Drem), (2, 1));
        assert_eq!(stack_effect(&Instruction::Ishl), (2, 1));
        assert_eq!(stack_effect(&Instruction::Lshl), (2, 1));
        assert_eq!(stack_effect(&Instruction::Ishr), (2, 1));
        assert_eq!(stack_effect(&Instruction::Lshr), (2, 1));
        assert_eq!(stack_effect(&Instruction::Iushr), (2, 1));
        assert_eq!(stack_effect(&Instruction::Lushr), (2, 1));
        assert_eq!(stack_effect(&Instruction::Iand), (2, 1));
        assert_eq!(stack_effect(&Instruction::Land), (2, 1));
        assert_eq!(stack_effect(&Instruction::Ior), (2, 1));
        assert_eq!(stack_effect(&Instruction::Lor), (2, 1));
        assert_eq!(stack_effect(&Instruction::Ixor), (2, 1));
        assert_eq!(stack_effect(&Instruction::Lxor), (2, 1));
        assert_eq!(stack_effect(&Instruction::Ineg), (1, 1));
        assert_eq!(stack_effect(&Instruction::Lneg), (1, 1));
        assert_eq!(stack_effect(&Instruction::Fneg), (1, 1));
        assert_eq!(
            stack_effect(&Instruction::IincW { index: 0, value: 1 }),
            (0, 0)
        );
        assert_eq!(stack_effect(&Instruction::I2f), (1, 1));
        assert_eq!(stack_effect(&Instruction::I2d), (1, 1));
        assert_eq!(stack_effect(&Instruction::L2i), (1, 1));
        assert_eq!(stack_effect(&Instruction::L2f), (1, 1));
        assert_eq!(stack_effect(&Instruction::L2d), (1, 1));
        assert_eq!(stack_effect(&Instruction::F2i), (1, 1));
        assert_eq!(stack_effect(&Instruction::F2l), (1, 1));
        assert_eq!(stack_effect(&Instruction::F2d), (1, 1));
        assert_eq!(stack_effect(&Instruction::D2i), (1, 1));
        assert_eq!(stack_effect(&Instruction::D2l), (1, 1));
        assert_eq!(stack_effect(&Instruction::D2f), (1, 1));
        assert_eq!(stack_effect(&Instruction::I2b), (1, 1));
        assert_eq!(stack_effect(&Instruction::I2c), (1, 1));
        assert_eq!(stack_effect(&Instruction::I2s), (1, 1));
        assert_eq!(stack_effect(&Instruction::Fcmpl), (2, 1));
        assert_eq!(stack_effect(&Instruction::Fcmpg), (2, 1));
        assert_eq!(stack_effect(&Instruction::Dcmpl), (2, 1));
        assert_eq!(stack_effect(&Instruction::Dcmpg), (2, 1));
        assert_eq!(stack_effect(&Instruction::Ifne(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::Iflt(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::Ifge(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::Ifgt(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::Ifle(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::Ifnull(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::Ifnonnull(0)), (1, 0));
        assert_eq!(stack_effect(&Instruction::IfIcmpeq(0)), (2, 0));
        assert_eq!(stack_effect(&Instruction::IfIcmplt(0)), (2, 0));
        assert_eq!(stack_effect(&Instruction::IfIcmpge(0)), (2, 0));
        assert_eq!(stack_effect(&Instruction::IfIcmpgt(0)), (2, 0));
        assert_eq!(stack_effect(&Instruction::IfIcmple(0)), (2, 0));
        assert_eq!(stack_effect(&Instruction::IfAcmpeq(0)), (2, 0));
        assert_eq!(stack_effect(&Instruction::IfAcmpne(0)), (2, 0));
        assert_eq!(stack_effect(&Instruction::GotoW(0)), (0, 0));
        assert_eq!(stack_effect(&Instruction::JsrW(0)), (0, 1));
        assert_eq!(stack_effect(&Instruction::Lreturn), (1, 0));
        assert_eq!(stack_effect(&Instruction::Freturn), (1, 0));
        assert_eq!(stack_effect(&Instruction::Dreturn), (1, 0));
        assert_eq!(stack_effect(&Instruction::Areturn), (1, 0));
        assert_eq!(
            stack_effect(&Instruction::Invokespecial(duke_classfile::CpIndex(1))),
            (1, 0)
        );
        assert_eq!(stack_effect(&Instruction::Monitorexit), (1, 0));

        assert_eq!(
            stack_effect(&Instruction::Tableswitch {
                default: 0,
                low: 0,
                high: 0,
                offsets: vec![],
            }),
            (1, 0)
        );
        assert_eq!(
            stack_effect(&Instruction::Lookupswitch {
                default: 0,
                pairs: vec![],
            }),
            (1, 0)
        );
        assert_eq!(
            stack_effect(&Instruction::Invokeinterface {
                index: duke_classfile::CpIndex(1),
                count: 1,
            }),
            (1, 0)
        );
        assert_eq!(
            stack_effect(&Instruction::Invokedynamic(duke_classfile::CpIndex(1))),
            (0, 0)
        );
        assert_eq!(
            stack_effect(&Instruction::Instanceof(duke_classfile::CpIndex(1))),
            (1, 1)
        );
        assert_eq!(
            stack_effect(&Instruction::Anewarray(duke_classfile::CpIndex(1))),
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
        assert_eq!(stack_effect(&Instruction::Imul), (2, 1));
        assert_eq!(stack_effect(&Instruction::DupX2), (3, 4));
        assert_eq!(stack_effect(&Instruction::Dup2X2), (4, 6));
        assert_eq!(stack_effect(&Instruction::Swap), (2, 2));
    }

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
}
