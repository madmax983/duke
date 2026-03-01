//! Switch-dispatch JVM bytecode interpreter for Duke Phase 4.
//!
//! Executes decoded instruction streams for methods containing integer, long,
//! float, and double arithmetic, control flow, and local variables.  Heap
//! allocation, field access, and method invocation are not yet implemented.

use std::collections::HashMap;

use duke_bytecode::Instruction;
use duke_classfile::types::CpEntry;
use duke_runtime::{Frame, Slot, VmError, VmResult};

/// Execute a decoded JVM instruction stream.
///
/// # Parameters
/// - `instructions`: output of `duke_bytecode::decode`
/// - `cp`: constant pool from the parsed `ClassFile`
/// - `args`: initial local variable values (method arguments)
/// - `max_stack`: `Code.max_stack` from the class file
/// - `max_locals`: `Code.max_locals` from the class file
///
/// # Returns
/// `Ok(Some(slot))` for value-returning methods, `Ok(None)` for `void`.
///
/// # Errors
/// Returns [`VmError`] on execution faults (division by zero, stack overflow,
/// unimplemented instruction, etc.).
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
pub fn execute(
    instructions: &[(usize, Instruction)],
    cp: &[Option<CpEntry>],
    args: Vec<Slot>,
    max_stack: u16,
    max_locals: u16,
) -> VmResult<Option<Slot>> {
    // Build PC → instruction-index map for O(1) branch resolution.
    let pc_to_idx: HashMap<usize, usize> =
        instructions.iter().enumerate().map(|(i, &(pc, _))| (pc, i)).collect();

    let mut frame = Frame::new(usize::from(max_stack), usize::from(max_locals), args)?;
    let mut idx: usize = 0;

    loop {
        let Some((pc, instr)) = instructions.get(idx) else {
            return Err(VmError::FellOffEnd);
        };
        let pc = *pc;

        // Jump to a PC-relative branch target (offset relative to current `pc`).
        macro_rules! jump {
            ($offset:expr) => {{
                let target = (pc as i64).wrapping_add(i64::from($offset)) as usize;
                idx = *pc_to_idx
                    .get(&target)
                    .ok_or(VmError::InvalidBranchTarget { pc: target })?;
                continue;
            }};
        }

        match instr {
            // ----------------------------------------------------------------
            // Constants
            // ----------------------------------------------------------------
            Instruction::Nop => {}
            Instruction::AconstNull => frame.push(Slot::Reference(None))?,
            Instruction::IconstM1 => frame.push(Slot::Int(-1))?,
            Instruction::Iconst0 => frame.push(Slot::Int(0))?,
            Instruction::Iconst1 => frame.push(Slot::Int(1))?,
            Instruction::Iconst2 => frame.push(Slot::Int(2))?,
            Instruction::Iconst3 => frame.push(Slot::Int(3))?,
            Instruction::Iconst4 => frame.push(Slot::Int(4))?,
            Instruction::Iconst5 => frame.push(Slot::Int(5))?,
            Instruction::Lconst0 => frame.push(Slot::Long(0))?,
            Instruction::Lconst1 => frame.push(Slot::Long(1))?,
            Instruction::Fconst0 => frame.push(Slot::Float(0.0))?,
            Instruction::Fconst1 => frame.push(Slot::Float(1.0))?,
            Instruction::Fconst2 => frame.push(Slot::Float(2.0))?,
            Instruction::Dconst0 => frame.push(Slot::Double(0.0))?,
            Instruction::Dconst1 => frame.push(Slot::Double(1.0))?,
            Instruction::Bipush(v) => frame.push(Slot::Int(i32::from(*v)))?,
            Instruction::Sipush(v) => frame.push(Slot::Int(i32::from(*v)))?,

            // ----------------------------------------------------------------
            // Constant pool load
            // ----------------------------------------------------------------
            Instruction::Ldc(raw_idx) => ldc_push(&mut frame, cp, usize::from(*raw_idx))?,
            Instruction::LdcW(cp_idx) | Instruction::Ldc2W(cp_idx) => {
                ldc_push(&mut frame, cp, usize::from(cp_idx.0))?;
            }

            // ----------------------------------------------------------------
            // Loads
            // ----------------------------------------------------------------
            Instruction::Iload(i)
            | Instruction::Lload(i)
            | Instruction::Fload(i)
            | Instruction::Dload(i)
            | Instruction::Aload(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }
            Instruction::Iload0
            | Instruction::Lload0
            | Instruction::Fload0
            | Instruction::Dload0
            | Instruction::Aload0 => {
                let s = frame.load_local(0)?;
                frame.push(s)?;
            }
            Instruction::Iload1
            | Instruction::Lload1
            | Instruction::Fload1
            | Instruction::Dload1
            | Instruction::Aload1 => {
                let s = frame.load_local(1)?;
                frame.push(s)?;
            }
            Instruction::Iload2
            | Instruction::Lload2
            | Instruction::Fload2
            | Instruction::Dload2
            | Instruction::Aload2 => {
                let s = frame.load_local(2)?;
                frame.push(s)?;
            }
            Instruction::Iload3
            | Instruction::Lload3
            | Instruction::Fload3
            | Instruction::Dload3
            | Instruction::Aload3 => {
                let s = frame.load_local(3)?;
                frame.push(s)?;
            }
            Instruction::IloadW(i)
            | Instruction::LloadW(i)
            | Instruction::FloadW(i)
            | Instruction::DloadW(i)
            | Instruction::AloadW(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }

            // ----------------------------------------------------------------
            // Stores
            // ----------------------------------------------------------------
            Instruction::Istore(i)
            | Instruction::Lstore(i)
            | Instruction::Fstore(i)
            | Instruction::Dstore(i)
            | Instruction::Astore(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }
            Instruction::Istore0
            | Instruction::Lstore0
            | Instruction::Fstore0
            | Instruction::Dstore0
            | Instruction::Astore0 => {
                let v = frame.pop()?;
                frame.store_local(0, v)?;
            }
            Instruction::Istore1
            | Instruction::Lstore1
            | Instruction::Fstore1
            | Instruction::Dstore1
            | Instruction::Astore1 => {
                let v = frame.pop()?;
                frame.store_local(1, v)?;
            }
            Instruction::Istore2
            | Instruction::Lstore2
            | Instruction::Fstore2
            | Instruction::Dstore2
            | Instruction::Astore2 => {
                let v = frame.pop()?;
                frame.store_local(2, v)?;
            }
            Instruction::Istore3
            | Instruction::Lstore3
            | Instruction::Fstore3
            | Instruction::Dstore3
            | Instruction::Astore3 => {
                let v = frame.pop()?;
                frame.store_local(3, v)?;
            }
            Instruction::IstoreW(i)
            | Instruction::LstoreW(i)
            | Instruction::FstoreW(i)
            | Instruction::DstoreW(i)
            | Instruction::AstoreW(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }

            // ----------------------------------------------------------------
            // Stack manipulation
            // ----------------------------------------------------------------
            Instruction::Pop => {
                frame.pop()?;
            }
            Instruction::Pop2 => {
                frame.pop()?;
                frame.pop()?;
            }
            Instruction::Dup => {
                let v = frame.pop()?;
                frame.push(v.clone())?;
                frame.push(v)?;
            }
            Instruction::Swap => {
                let a = frame.pop()?;
                let b = frame.pop()?;
                frame.push(a)?;
                frame.push(b)?;
            }

            // ----------------------------------------------------------------
            // Integer arithmetic
            // ----------------------------------------------------------------
            Instruction::Iadd => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_add(b)))?;
            }
            Instruction::Isub => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_sub(b)))?;
            }
            Instruction::Imul => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_mul(b)))?;
            }
            Instruction::Idiv => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_div(b)))?;
            }
            Instruction::Irem => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_rem(b)))?;
            }
            Instruction::Ineg => {
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_neg()))?;
            }
            Instruction::Ishl => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shl((s & 0x1F) as u32)))?;
            }
            Instruction::Ishr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shr((s & 0x1F) as u32)))?;
            }
            Instruction::Iushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(((a as u32) >> (s as u32 & 0x1F)) as i32))?;
            }
            Instruction::Iand => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a & b))?;
            }
            Instruction::Ior => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a | b))?;
            }
            Instruction::Ixor => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a ^ b))?;
            }
            Instruction::Iinc { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }
            Instruction::IincW { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }

            // ----------------------------------------------------------------
            // Long arithmetic
            // ----------------------------------------------------------------
            Instruction::Ladd => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_add(b)))?;
            }
            Instruction::Lsub => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_sub(b)))?;
            }
            Instruction::Lmul => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_mul(b)))?;
            }
            Instruction::Ldiv => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_div(b)))?;
            }
            Instruction::Lrem => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_rem(b)))?;
            }
            Instruction::Lneg => {
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_neg()))?;
            }
            Instruction::Lshl => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shl((s & 0x3F) as u32)))?;
            }
            Instruction::Lshr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shr((s & 0x3F) as u32)))?;
            }
            Instruction::Lushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(((a as u64) >> (s as u32 & 0x3F)) as i64))?;
            }
            Instruction::Land => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a & b))?;
            }
            Instruction::Lor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a | b))?;
            }
            Instruction::Lxor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a ^ b))?;
            }
            Instruction::Lcmp => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                let r = match a.cmp(&b) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Float arithmetic
            // ----------------------------------------------------------------
            Instruction::Fadd => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a + b))?;
            }
            Instruction::Fsub => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a - b))?;
            }
            Instruction::Fmul => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a * b))?;
            }
            Instruction::Fdiv => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a / b))?;
            }
            Instruction::Frem => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a % b))?;
            }
            Instruction::Fneg => {
                let a = frame.pop_float()?;
                frame.push(Slot::Float(-a))?;
            }
            Instruction::Fcmpl | Instruction::Fcmpg => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Fcmpg) {
                    1 // NaN result: Fcmpg pushes 1, Fcmpl pushes -1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Double arithmetic
            // ----------------------------------------------------------------
            Instruction::Dadd => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a + b))?;
            }
            Instruction::Dsub => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a - b))?;
            }
            Instruction::Dmul => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a * b))?;
            }
            Instruction::Ddiv => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a / b))?;
            }
            Instruction::Drem => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a % b))?;
            }
            Instruction::Dneg => {
                let a = frame.pop_double()?;
                frame.push(Slot::Double(-a))?;
            }
            Instruction::Dcmpl | Instruction::Dcmpg => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Dcmpg) {
                    1 // NaN result: Dcmpg pushes 1, Dcmpl pushes -1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Type conversions
            // ----------------------------------------------------------------
            Instruction::I2l => {
                let v = frame.pop_int()?;
                frame.push(Slot::Long(i64::from(v)))?;
            }
            Instruction::I2f => {
                let v = frame.pop_int()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2d => {
                let v = frame.pop_int()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::L2i => {
                let v = frame.pop_long()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::L2f => {
                let v = frame.pop_long()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::L2d => {
                let v = frame.pop_long()?;
                frame.push(Slot::Double(v as f64))?;
            }
            Instruction::F2i => {
                let v = frame.pop_float()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::F2l => {
                let v = frame.pop_float()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::F2d => {
                let v = frame.pop_float()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::D2i => {
                let v = frame.pop_double()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::D2l => {
                let v = frame.pop_double()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::D2f => {
                let v = frame.pop_double()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2b => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(v as i8 as i32))?;
            }
            Instruction::I2c => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(v as u16 as i32))?;
            }
            Instruction::I2s => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(v as i16 as i32))?;
            }

            // ----------------------------------------------------------------
            // Returns
            // ----------------------------------------------------------------
            Instruction::Return => return Ok(None),
            Instruction::Ireturn => return Ok(Some(Slot::Int(frame.pop_int()?))),
            Instruction::Lreturn => return Ok(Some(Slot::Long(frame.pop_long()?))),
            Instruction::Freturn => return Ok(Some(Slot::Float(frame.pop_float()?))),
            Instruction::Dreturn => return Ok(Some(Slot::Double(frame.pop_double()?))),

            // ----------------------------------------------------------------
            // Branches
            // ----------------------------------------------------------------
            Instruction::Goto(offset) => jump!(*offset),
            Instruction::GotoW(offset) => jump!(*offset),

            Instruction::Ifeq(offset) => {
                if frame.pop_int()? == 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifne(offset) => {
                if frame.pop_int()? != 0 {
                    jump!(*offset);
                }
            }
            Instruction::Iflt(offset) => {
                if frame.pop_int()? < 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifge(offset) => {
                if frame.pop_int()? >= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifgt(offset) => {
                if frame.pop_int()? > 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifle(offset) => {
                if frame.pop_int()? <= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifnull(offset) => {
                if matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::Ifnonnull(offset) => {
                if !matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpeq(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpne(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a != b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmplt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a < b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpge(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a >= b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpgt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a > b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmple(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a <= b {
                    jump!(*offset);
                }
            }
            // Reference comparisons: stub — just pop values (no heap in Phase 4).
            Instruction::IfAcmpeq(_) | Instruction::IfAcmpne(_) => {
                frame.pop()?;
                frame.pop()?;
            }

            other => {
                return Err(VmError::Unimplemented { mnemonic: other.mnemonic() });
            }
        }

        idx += 1;
    }
}

/// Push a constant pool value onto the frame's operand stack.
fn ldc_push(frame: &mut Frame, cp: &[Option<CpEntry>], idx: usize) -> VmResult<()> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Integer(v)) => frame.push(Slot::Int(*v)),
        Some(CpEntry::Float(v)) => frame.push(Slot::Float(*v)),
        Some(CpEntry::Long(v)) => frame.push(Slot::Long(*v)),
        Some(CpEntry::Double(v)) => frame.push(Slot::Double(*v)),
        _ => Err(VmError::InvalidCpIndex { index: idx }),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Unit tests: hand-crafted instruction streams ----

    #[test]
    fn execute_iconst_ireturn() {
        let instructions = vec![(0, Instruction::Iconst1), (1, Instruction::Ireturn)];
        let result = execute(&instructions, &[], vec![], 2, 1).expect("should execute");
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn execute_bipush_istore_iload() {
        // bipush 42, istore_0, iload_0, ireturn
        let instructions = vec![
            (0, Instruction::Bipush(42)),
            (2, Instruction::Istore0),
            (3, Instruction::Iload0),
            (4, Instruction::Ireturn),
        ];
        let result = execute(&instructions, &[], vec![], 2, 2).unwrap();
        assert_eq!(result, Some(Slot::Int(42)));
    }

    #[test]
    fn execute_loads_args() {
        // iload_0, iload_1, pop, ireturn — returns first arg
        let instructions = vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::Pop),
            (3, Instruction::Ireturn),
        ];
        let result =
            execute(&instructions, &[], vec![Slot::Int(99), Slot::Int(0)], 2, 2).unwrap();
        assert_eq!(result, Some(Slot::Int(99)));
    }

    #[test]
    fn execute_sipush() {
        let instructions = vec![(0, Instruction::Sipush(1000)), (3, Instruction::Ireturn)];
        let result = execute(&instructions, &[], vec![], 2, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(1000)));
    }

    #[test]
    fn hand_coded_add() {
        // iload_0, iload_1, iadd, ireturn
        let instrs = vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::Iadd),
            (3, Instruction::Ireturn),
        ];
        let result =
            execute(&instrs, &[], vec![Slot::Int(3), Slot::Int(4)], 2, 2).unwrap();
        assert_eq!(result, Some(Slot::Int(7)));
    }

    #[test]
    fn hand_coded_div_by_zero() {
        let instrs = vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::Idiv),
            (3, Instruction::Ireturn),
        ];
        let err =
            execute(&instrs, &[], vec![Slot::Int(10), Slot::Int(0)], 2, 2).unwrap_err();
        assert!(matches!(err, VmError::DivisionByZero));
    }

    #[test]
    fn hand_coded_long_add() {
        let instrs = vec![
            (0, Instruction::Lload0),
            (1, Instruction::Lload1),
            (2, Instruction::Ladd),
            (3, Instruction::Lreturn),
        ];
        let result = execute(
            &instrs,
            &[],
            vec![Slot::Long(1_000_000_000), Slot::Long(2_000_000_000)],
            2,
            2,
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Long(3_000_000_000)));
    }

    #[test]
    fn hand_coded_ifeq_taken() {
        // iconst_0 (pc=0), ifeq +5 (pc=1, target=6), iconst_1 (pc=4), ireturn (pc=5),
        // iconst_2 (pc=6), ireturn (pc=7)
        let instrs = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Ifeq(5)), // 0 == 0, jump to pc=6
            (4, Instruction::Iconst1),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst2),
            (7, Instruction::Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    #[test]
    fn hand_coded_ifeq_not_taken() {
        let instrs = vec![
            (0, Instruction::Iconst1),  // push 1
            (1, Instruction::Ifeq(5)),  // 1 != 0, NOT taken
            (4, Instruction::Iconst1),  // reached
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst2),
            (7, Instruction::Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn hand_coded_goto() {
        // iconst_5 (pc=0), goto +4 (pc=1, target=5), pop (pc=4, skipped),
        // ireturn (pc=5) — returns 5
        let instrs = vec![
            (0, Instruction::Iconst5),
            (1, Instruction::Goto(4)), // jump to pc=5
            (4, Instruction::Pop),     // skipped
            (5, Instruction::Ireturn), // returns 5
        ];
        let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(5)));
    }

    // ---- Integration tests: load Arithmetic.class and execute real bytecode ----

    fn fixture(name: &str) -> std::path::PathBuf {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../tests/fixtures");
        p.push(name);
        p
    }

    /// Parse and execute a static method from a `.class` file that takes `i32` args
    /// and returns an `i32`.
    fn run_static_int(class_name: &str, method_name: &str, args: Vec<i32>) -> i32 {
        use duke_bytecode::decode;
        use duke_classfile::{
            parse,
            types::{AttributeData, CpEntry},
        };

        let bytes = std::fs::read(fixture(class_name)).expect("fixture not found");
        let cf = parse(&bytes).expect("parse failed");

        let method = cf
            .methods
            .iter()
            .find(|m| {
                let Some(Some(CpEntry::Utf8(s))) =
                    cf.constant_pool.get(m.name_index.0 as usize)
                else {
                    return false;
                };
                s.as_str() == method_name
            })
            .unwrap_or_else(|| panic!("method '{method_name}' not found"));

        let code = method
            .attributes
            .iter()
            .find_map(|a| if let AttributeData::Code(c) = &a.data { Some(c) } else { None })
            .expect("no Code attribute");

        let instructions = decode(&code.code).expect("decode failed");
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();

        match execute(&instructions, &cf.constant_pool, slots, code.max_stack, code.max_locals)
            .expect("execute failed")
        {
            Some(Slot::Int(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    // Basic arithmetic
    #[test]
    fn int_add() {
        assert_eq!(run_static_int("Arithmetic.class", "add", vec![3, 4]), 7);
    }
    #[test]
    fn int_subtract() {
        assert_eq!(run_static_int("Arithmetic.class", "subtract", vec![10, 3]), 7);
    }
    #[test]
    fn int_multiply() {
        assert_eq!(run_static_int("Arithmetic.class", "multiply", vec![3, 4]), 12);
    }
    #[test]
    fn int_divide() {
        assert_eq!(run_static_int("Arithmetic.class", "divide", vec![10, 2]), 5);
    }
    #[test]
    fn int_remainder() {
        assert_eq!(run_static_int("Arithmetic.class", "remainder", vec![10, 3]), 1);
    }
    #[test]
    fn int_negate() {
        assert_eq!(run_static_int("Arithmetic.class", "negate", vec![-5]), 5);
    }
    #[test]
    fn int_shift_left() {
        assert_eq!(run_static_int("Arithmetic.class", "shiftLeft", vec![1, 4]), 16);
    }
    #[test]
    fn int_bitwise_and() {
        assert_eq!(
            run_static_int("Arithmetic.class", "bitwiseAnd", vec![0b1111, 0b1010]),
            0b1010
        );
    }
    #[test]
    fn int_bitwise_or() {
        assert_eq!(
            run_static_int("Arithmetic.class", "bitwiseOr", vec![0b1111, 0b1010]),
            0b1111
        );
    }

    // Conditionals
    #[test]
    fn int_max_a_wins() {
        assert_eq!(run_static_int("Arithmetic.class", "max", vec![7, 3]), 7);
    }
    #[test]
    fn int_max_b_wins() {
        assert_eq!(run_static_int("Arithmetic.class", "max", vec![3, 7]), 7);
    }
    #[test]
    fn int_abs_neg() {
        assert_eq!(run_static_int("Arithmetic.class", "abs", vec![-5]), 5);
    }
    #[test]
    fn int_abs_pos() {
        assert_eq!(run_static_int("Arithmetic.class", "abs", vec![5]), 5);
    }
    #[test]
    fn int_clamp_mid() {
        assert_eq!(run_static_int("Arithmetic.class", "clamp", vec![5, 1, 10]), 5);
    }
    #[test]
    fn int_clamp_lo() {
        assert_eq!(run_static_int("Arithmetic.class", "clamp", vec![0, 1, 10]), 1);
    }
    #[test]
    fn int_clamp_hi() {
        assert_eq!(run_static_int("Arithmetic.class", "clamp", vec![15, 1, 10]), 10);
    }

    // Control flow / loops
    #[test]
    fn int_factorial_0() {
        assert_eq!(run_static_int("Arithmetic.class", "factorial", vec![0]), 1);
    }
    #[test]
    fn int_factorial_5() {
        assert_eq!(run_static_int("Arithmetic.class", "factorial", vec![5]), 120);
    }
    #[test]
    fn int_factorial_10() {
        assert_eq!(run_static_int("Arithmetic.class", "factorial", vec![10]), 3_628_800);
    }
    #[test]
    fn int_fibonacci_0() {
        assert_eq!(run_static_int("Arithmetic.class", "fibonacci", vec![0]), 0);
    }
    #[test]
    fn int_fibonacci_1() {
        assert_eq!(run_static_int("Arithmetic.class", "fibonacci", vec![1]), 1);
    }
    #[test]
    fn int_fibonacci_10() {
        assert_eq!(run_static_int("Arithmetic.class", "fibonacci", vec![10]), 55);
    }
    #[test]
    fn int_sum_to_100() {
        assert_eq!(run_static_int("Arithmetic.class", "sumTo", vec![100]), 5050);
    }
}
