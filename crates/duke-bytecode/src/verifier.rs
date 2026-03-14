//! Structural bytecode verifier (JVM §4.10 subset).
#![allow(clippy::missing_const_for_fn, clippy::too_many_lines)]
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
/// # Parameters
///
/// - `instructions`: output of [`crate::decoder::decode`]
/// - `max_stack`: from the `Code` attribute
/// - `max_locals`: from the `Code` attribute
///
/// # Errors
///
/// Returns the first [`VerifyError`] encountered.
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

        let (pops, pushes) = stack_effect(instr);

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
        if is_return(instr) && depth != 0 {
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

#[allow(clippy::match_same_arms)]
fn stack_effect(instr: &Instruction) -> (usize, usize) {
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

fn is_return(instr: &Instruction) -> bool {
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
fn check_locals(instr: &Instruction, pc: usize, max_locals: usize) -> VerifyResult<()> {
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
        return Err(VerifyError::LocalOutOfBounds {
            pc,
            index: i,
            max_locals,
        });
    }
    Ok(())
}
