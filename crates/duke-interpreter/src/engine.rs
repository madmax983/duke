use crate::context::{ClassContext, ExceptionEntry, FieldEntry, MethodEntry};
use crate::registry::{
    CallbackOps, ClassRegistry, HandlerKind, LambdaInfo, NativeControl, NativeThreadAction,
    ReflectedClassInfo, ReflectedFieldInfo, ReflectedMethodInfo,
};
use crate::threading::{ThreadRecord, ThreadRuntime};
use crate::{THREAD_ID_SLOT, THREAD_TARGET_SLOT, execute_string_concat_recipe};
use duke_bytecode::Instruction;
use duke_bytecode::instruction::ArrayType;
use duke_classfile::types::CpEntry;
use duke_loader::ClassLoader;
use duke_runtime::{Frame, Slot, VmError, VmResult};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;

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
    let pc_to_idx: HashMap<usize, usize> = instructions
        .iter()
        .enumerate()
        .map(|(i, &(pc, _))| (pc, i))
        .collect();

    let mut frame = Frame::new(usize::from(max_stack), usize::from(max_locals), args)?;
    let mut idx: usize = 0;
    // Local heap for array objects allocated during single-method execution.
    let mut local_heap: Vec<(String, Vec<Slot>, Option<String>)> = Vec::new();
    let mut string_intern: HashMap<usize, u64> = HashMap::new();

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
            Instruction::Ldc(raw_idx) => {
                let cp_idx = usize::from(*raw_idx);
                if let Some(CpEntry::String { string_index }) =
                    cp.get(cp_idx).and_then(|e| e.as_ref())
                {
                    let si = string_index.0 as usize;
                    let r = if let Some(&cached) = string_intern.get(&cp_idx) {
                        cached
                    } else {
                        let s = match cp.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        let r = local_heap.len() as u64;
                        local_heap.push(("java/lang/String".to_string(), Vec::new(), Some(s)));
                        string_intern.insert(cp_idx, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    ldc_push(&mut frame, cp, cp_idx)?;
                }
            }
            Instruction::LdcW(cp_idx) | Instruction::Ldc2W(cp_idx) => {
                let idx_val = usize::from(cp_idx.0);
                if let Some(CpEntry::String { string_index }) =
                    cp.get(idx_val).and_then(|e| e.as_ref())
                {
                    let si = string_index.0 as usize;
                    let r = if let Some(&cached) = string_intern.get(&idx_val) {
                        cached
                    } else {
                        let s = match cp.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        let r = local_heap.len() as u64;
                        local_heap.push(("java/lang/String".to_string(), Vec::new(), Some(s)));
                        string_intern.insert(idx_val, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    ldc_push(&mut frame, cp, idx_val)?;
                }
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
                frame.push(v)?;
                frame.push(v)?;
            }
            Instruction::DupX1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::DupX2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                let v4 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v4)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
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
                // Java semantics: exact bit equality for 0 check; NaN handled separately.
                #[allow(clippy::float_cmp)]
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
                // Java semantics: exact bit equality for 0 check; NaN handled separately.
                #[allow(clippy::float_cmp)]
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
                frame.push(Slot::Int(i32::from(v as i8)))?;
            }
            Instruction::I2c => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as u16)))?;
            }
            Instruction::I2s => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as i16)))?;
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
            // Reference comparisons
            Instruction::IfAcmpeq(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfAcmpne(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a != b {
                    jump!(*offset);
                }
            }

            // ----------------------------------------------------------------
            // Array allocation
            // ----------------------------------------------------------------
            Instruction::Newarray(array_type) => {
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let class_name = match array_type {
                    ArrayType::Boolean => "[Z",
                    ArrayType::Char => "[C",
                    ArrayType::Float => "[F",
                    ArrayType::Double => "[D",
                    ArrayType::Byte => "[B",
                    ArrayType::Short => "[S",
                    ArrayType::Int => "[I",
                    ArrayType::Long => "[J",
                };
                let init_slot = match array_type {
                    ArrayType::Long => Slot::Long(0),
                    ArrayType::Float => Slot::Float(0.0),
                    ArrayType::Double => Slot::Double(0.0),
                    _ => Slot::Int(0),
                };
                let r = local_heap.len() as u64;
                local_heap.push((
                    class_name.to_string(),
                    vec![init_slot; count as usize],
                    None,
                ));
                frame.push(Slot::Reference(Some(r)))?;
            }
            Instruction::Anewarray(cp_idx) => {
                let element_type = resolve_class_name(cp, usize::from(cp_idx.0))?;
                let array_type = format!("[L{element_type};");
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let r = local_heap.len() as u64;
                local_heap.push((
                    array_type,
                    vec![Slot::Reference(None); count as usize],
                    None,
                ));
                frame.push(Slot::Reference(Some(r)))?;
            }
            Instruction::Arraylength => {
                let r = frame.pop_ref()?;
                let len = local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1
                    .len();
                frame.push(Slot::Int(len as i32))?;
            }

            // ---- Int array ----
            Instruction::Iaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_int()?;
                frame.push(Slot::Int(v))?;
            }
            Instruction::Iastore => {
                let val = frame.pop_int()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Long array ----
            Instruction::Laload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_long()?;
                frame.push(Slot::Long(v))?;
            }
            Instruction::Lastore => {
                let val = frame.pop_long()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Long(val);
            }

            // ---- Float array ----
            Instruction::Faload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_float()?;
                frame.push(Slot::Float(v))?;
            }
            Instruction::Fastore => {
                let val = frame.pop_float()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Float(val);
            }

            // ---- Double array ----
            Instruction::Daload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_double()?;
                frame.push(Slot::Double(v))?;
            }
            Instruction::Dastore => {
                let val = frame.pop_double()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Double(val);
            }

            // ---- Reference array ----
            Instruction::Aaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize];
                frame.push(v)?;
            }
            Instruction::Aastore => {
                let val = frame.pop()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = val;
            }

            // ---- Byte/boolean array (stored as Int, truncated to i8) ----
            Instruction::Baload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as i8);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Bastore => {
                let val = i32::from(frame.pop_int()? as i8);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Char array (stored as Int, masked to u16) ----
            Instruction::Caload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as u16);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Castore => {
                let val = i32::from(frame.pop_int()? as u16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Short array (stored as Int, truncated to i16) ----
            Instruction::Saload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as i16);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Sastore => {
                let val = i32::from(frame.pop_int()? as i16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Switch ----
            Instruction::Tableswitch {
                default,
                low,
                high,
                offsets,
            } => {
                let key = frame.pop_int()?;
                let offset = if key >= *low && key <= *high {
                    offsets[(key - low) as usize]
                } else {
                    *default
                };
                jump!(offset);
            }
            Instruction::Lookupswitch { default, pairs } => {
                let key = frame.pop_int()?;
                let offset = pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map_or(*default, |(_, off)| *off);
                jump!(offset);
            }

            // ---- checkcast / instanceof ----
            Instruction::Checkcast(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(slot)?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = resolve_class_name(cp, usize::from(cp_idx.0))?;
                        let actual = local_heap
                            .get(*r as usize)
                            .ok_or(VmError::InvalidRef { address: *r })?
                            .0
                            .clone();
                        if actual == target {
                            frame.push(slot)?;
                        } else {
                            return Err(VmError::ClassCastException {
                                from: actual,
                                to: target,
                            });
                        }
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }
            Instruction::Instanceof(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(Slot::Int(0))?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = resolve_class_name(cp, usize::from(cp_idx.0))?;
                        let actual = local_heap
                            .get(*r as usize)
                            .ok_or(VmError::InvalidRef { address: *r })?
                            .0
                            .clone();
                        if actual == target {
                            frame.push(Slot::Int(1))?;
                        } else {
                            frame.push(Slot::Int(0))?;
                        }
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }

            // ---- athrow ----
            Instruction::Athrow => {
                let r = frame.pop_ref()?;
                let class_name = local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .0
                    .clone();
                return Err(VmError::JavaException { class_name });
            }

            // ---- monitor (no-op, single-threaded) ----
            Instruction::Monitorenter | Instruction::Monitorexit => {
                let _obj = frame.pop()?;
            }

            other => {
                return Err(VmError::Unimplemented {
                    mnemonic: other.mnemonic(),
                });
            }
        }

        idx += 1;
    }
}

/// Ensure a class is initialized. Runs `<clinit>` if present and not yet run.
///
/// Must be called before first active use of a class (new, getstatic, putstatic, invokestatic).
/// `triggered_by` names the class that caused this init (empty string for the entry-point class).
#[allow(clippy::used_underscore_binding, clippy::cast_possible_truncation)]
fn ensure_initialized(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    _triggered_by: &str,
) -> VmResult<()> {
    if registry.is_initialized(class_name) {
        return Ok(());
    }
    // Mark as initialized BEFORE running clinit to prevent infinite recursion.
    registry.mark_initialized(class_name);

    // Check if the class has a <clinit> method.
    let has_clinit = registry.get(class_name).is_ok_and(|ctx| {
        ctx.methods
            .iter()
            .any(|m| m.name == "<clinit>" && m.descriptor == "()V")
    });

    if has_clinit {
        // Run <clinit> by calling it through execute_class.
        #[cfg(feature = "telemetry")]
        #[allow(clippy::used_underscore_binding)]
        let _clinit_start = std::time::Instant::now();
        execute_class(
            registry,
            loader,
            heap,
            stdout,
            class_name,
            "<clinit>",
            "()V",
            &[],
        )?;
        #[cfg(feature = "telemetry")]
        registry.telemetry.class_init_dag.record(
            class_name,
            _triggered_by,
            _clinit_start.elapsed().as_nanos() as u64,
        );
    }
    Ok(())
}

/// Reusable pool of frame backing buffers.
///
/// Eliminates per-call `Vec<Slot>` allocation for locals and operand stack.
/// On a recursive workload the pool reaches steady state after the first
/// call-depth calls; all subsequent frames are pool hits with zero allocation.
///
/// The pool is private to a single `execute_class` invocation.
pub(crate) struct FramePool {
    pub(crate) free: Vec<(Vec<Slot>, Vec<Slot>)>,
}

impl FramePool {
    pub(crate) const fn new() -> Self {
        Self { free: Vec::new() }
    }

    /// Acquire a `(locals_buf, stack_buf)` pair.
    /// Returns a pooled pair if available, otherwise allocates fresh Vecs.
    pub(crate) fn acquire(&mut self) -> (Vec<Slot>, Vec<Slot>) {
        self.free.pop().unwrap_or_default()
    }

    /// Return buffers to the pool.
    /// `stack` must already be empty (guaranteed by `Frame::into_pool_bufs`).
    /// Caps pool at 256 entries to bound memory usage.
    pub(crate) fn release(&mut self, locals: Vec<Slot>, stack: Vec<Slot>) {
        // Cap at 256 entries — typical max call depth is well under 100; anything
        // beyond this is dead weight. Entries dropped here are freed to the allocator.
        if self.free.len() < 256 {
            self.free.push((locals, stack));
        }
    }
}

struct ExecutionState {
    current_class: String,
    method_idx: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: std::sync::Arc<[(usize, Instruction)]>,
    frame: Frame,
    call_stack: Vec<CallFrame>,
    frame_pool: FramePool,
    dispatch_cache: HashMap<String, HashMap<u16, (String, usize, usize)>>,
    idx: usize,
    string_intern: HashMap<usize, u64>,
    #[cfg(feature = "telemetry")]
    current_method: String,
}

struct InterpreterCallbackOps<'a> {
    registry: &'a mut ClassRegistry,
    loader: &'a dyn ClassLoader,
}

impl CallbackOps for InterpreterCallbackOps<'_> {
    fn invoke(
        &mut self,
        heap: &mut duke_gc::Heap,
        output: &mut dyn Write,
        class: &str,
        method: &str,
        descriptor: &str,
        args: Vec<Slot>,
    ) -> VmResult<Option<Slot>> {
        execute_class(
            self.registry,
            self.loader,
            heap,
            output,
            class,
            method,
            descriptor,
            &args,
        )
    }

    fn ensure_loaded(&mut self, class: &str) -> VmResult<()> {
        if self.registry.ensure_loaded(class, self.loader)? {
            Ok(())
        } else {
            Err(VmError::ClassNotFound {
                name: class.to_string(),
            })
        }
    }

    fn inspect_class(&mut self, class: &str) -> VmResult<ReflectedClassInfo> {
        inspect_reflected_class(self.registry, self.loader, class)
    }
}

impl ExecutionState {
    fn new(
        registry: &ClassRegistry,
        class_name: &str,
        #[cfg_attr(not(feature = "telemetry"), allow(unused_variables))] method_name: &str,
        entry_idx: usize,
        args: &[Slot],
    ) -> VmResult<Self> {
        let current_class = class_name.to_string();
        let pc_to_idx = {
            let ctx = registry.get(&current_class)?;
            std::sync::Arc::clone(&ctx.methods[entry_idx].pc_to_idx)
        };
        let frame = {
            let ctx = registry.get(&current_class)?;
            Frame::new(
                usize::from(ctx.methods[entry_idx].max_stack),
                usize::from(ctx.methods[entry_idx].max_locals),
                args.to_vec(),
            )?
        };
        let instructions = {
            let ctx = registry.get(&current_class)?;
            std::sync::Arc::clone(&ctx.methods[entry_idx].instructions)
        };

        Ok(Self {
            current_class,
            method_idx: entry_idx,
            pc_to_idx,
            instructions,
            frame,
            call_stack: Vec::new(),
            frame_pool: FramePool::new(),
            dispatch_cache: HashMap::new(),
            idx: 0,
            string_intern: HashMap::new(),
            #[cfg(feature = "telemetry")]
            current_method: method_name.to_string(),
        })
    }
}

#[cfg(feature = "telemetry")]
fn refresh_current_method_name(
    current_method: &mut String,
    registry: &ClassRegistry,
    current_class: &str,
    method_idx: usize,
) {
    *current_method = registry
        .get(current_class)
        .map(|c| {
            c.methods
                .get(method_idx)
                .map(|m| m.name.clone())
                .unwrap_or_default()
        })
        .unwrap_or_default();
}

#[allow(clippy::too_many_arguments)]
fn activate_method_state(
    frame: &mut Frame,
    method_idx: &mut usize,
    pc_to_idx: &mut std::sync::Arc<std::collections::HashMap<usize, usize>>,
    instructions: &mut std::sync::Arc<[(usize, Instruction)]>,
    current_class: &mut String,
    call_stack: &mut Vec<CallFrame>,
    registry: &ClassRegistry,
    callee_class: String,
    callee_idx: usize,
    callee_pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    callee_frame: Frame,
    resume_idx: usize,
    #[cfg(feature = "telemetry")] current_method: &mut String,
) -> VmResult<()> {
    call_stack.push(CallFrame {
        frame: std::mem::replace(frame, callee_frame),
        method_idx: *method_idx,
        pc_to_idx: std::sync::Arc::clone(pc_to_idx),
        resume_idx,
        class_name: current_class.clone(),
    });
    *method_idx = callee_idx;
    *pc_to_idx = callee_pc_to_idx;
    *current_class = callee_class;
    *instructions =
        std::sync::Arc::clone(&registry.get(current_class)?.methods[*method_idx].instructions);
    #[cfg(feature = "telemetry")]
    refresh_current_method_name(current_method, registry, current_class, *method_idx);
    Ok(())
}

enum ExecutionOutcome {
    Returned(Option<Slot>),
    ThreadAction(NativeThreadAction),
}

fn finish_native_call(
    native_control: &mut NativeControl,
    frame: &mut Frame,
    idx: &mut usize,
    result: Option<Slot>,
) -> VmResult<Option<ExecutionOutcome>> {
    if let Some(val) = result {
        frame.push(val)?;
    }
    *idx += 1;
    if let Some(action) = native_control.take() {
        return Ok(Some(ExecutionOutcome::ThreadAction(action)));
    }
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
fn prepare_execution_state(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<ExecutionState> {
    let entry_idx = {
        let ctx = registry.get(class_name)?;
        ctx.methods
            .iter()
            .position(|m| m.name == method_name && m.descriptor == descriptor)
            .ok_or_else(|| VmError::MethodNotFound {
                name: method_name.to_string(),
                descriptor: descriptor.to_string(),
            })?
    };

    ensure_initialized(registry, loader, heap, stdout, class_name, "")?;
    ExecutionState::new(registry, class_name, method_name, entry_idx, args)
}

#[cfg(feature = "telemetry")]
#[allow(clippy::too_many_lines)]
const fn instr_name(instr: &duke_bytecode::Instruction) -> &'static str {
    use duke_bytecode::Instruction as I;
    match instr {
        I::Nop => "nop",
        I::AconstNull => "aconst_null",
        I::Iconst0 | I::Iconst1 | I::Iconst2 | I::Iconst3 | I::Iconst4 | I::Iconst5 => "iconst_n",
        I::IconstM1 => "iconst_m1",
        I::Lconst0 | I::Lconst1 => "lconst_n",
        I::Fconst0 | I::Fconst1 | I::Fconst2 => "fconst_n",
        I::Dconst0 | I::Dconst1 => "dconst_n",
        I::Bipush(_) => "bipush",
        I::Sipush(_) => "sipush",
        I::Ldc(_) | I::LdcW(_) | I::Ldc2W(_) => "ldc",
        I::Iload(_) | I::Iload0 | I::Iload1 | I::Iload2 | I::Iload3 | I::IloadW(_) => "iload",
        I::Lload(_) | I::Lload0 | I::Lload1 | I::Lload2 | I::Lload3 | I::LloadW(_) => "lload",
        I::Fload(_) | I::Fload0 | I::Fload1 | I::Fload2 | I::Fload3 | I::FloadW(_) => "fload",
        I::Dload(_) | I::Dload0 | I::Dload1 | I::Dload2 | I::Dload3 | I::DloadW(_) => "dload",
        I::Aload(_) | I::Aload0 | I::Aload1 | I::Aload2 | I::Aload3 | I::AloadW(_) => "aload",
        I::Istore(_) | I::Istore0 | I::Istore1 | I::Istore2 | I::Istore3 | I::IstoreW(_) => {
            "istore"
        }
        I::Lstore(_) | I::Lstore0 | I::Lstore1 | I::Lstore2 | I::Lstore3 | I::LstoreW(_) => {
            "lstore"
        }
        I::Fstore(_) | I::Fstore0 | I::Fstore1 | I::Fstore2 | I::Fstore3 | I::FstoreW(_) => {
            "fstore"
        }
        I::Dstore(_) | I::Dstore0 | I::Dstore1 | I::Dstore2 | I::Dstore3 | I::DstoreW(_) => {
            "dstore"
        }
        I::Astore(_) | I::Astore0 | I::Astore1 | I::Astore2 | I::Astore3 | I::AstoreW(_) => {
            "astore"
        }
        I::Iaload => "iaload",
        I::Laload => "laload",
        I::Faload => "faload",
        I::Daload => "daload",
        I::Aaload => "aaload",
        I::Baload => "baload",
        I::Caload => "caload",
        I::Saload => "saload",
        I::Iastore => "iastore",
        I::Lastore => "lastore",
        I::Fastore => "fastore",
        I::Dastore => "dastore",
        I::Aastore => "aastore",
        I::Bastore => "bastore",
        I::Castore => "castore",
        I::Sastore => "sastore",
        I::Pop => "pop",
        I::Pop2 => "pop2",
        I::Dup => "dup",
        I::DupX1 => "dup_x1",
        I::DupX2 => "dup_x2",
        I::Dup2 => "dup2",
        I::Dup2X1 => "dup2_x1",
        I::Dup2X2 => "dup2_x2",
        I::Swap => "swap",
        I::Iadd => "iadd",
        I::Ladd => "ladd",
        I::Fadd => "fadd",
        I::Dadd => "dadd",
        I::Isub => "isub",
        I::Lsub => "lsub",
        I::Fsub => "fsub",
        I::Dsub => "dsub",
        I::Imul => "imul",
        I::Lmul => "lmul",
        I::Fmul => "fmul",
        I::Dmul => "dmul",
        I::Idiv => "idiv",
        I::Ldiv => "ldiv",
        I::Fdiv => "fdiv",
        I::Ddiv => "ddiv",
        I::Irem => "irem",
        I::Lrem => "lrem",
        I::Frem => "frem",
        I::Drem => "drem",
        I::Ineg => "ineg",
        I::Lneg => "lneg",
        I::Fneg => "fneg",
        I::Dneg => "dneg",
        I::Ishl => "ishl",
        I::Lshl => "lshl",
        I::Ishr => "ishr",
        I::Lshr => "lshr",
        I::Iushr => "iushr",
        I::Lushr => "lushr",
        I::Iand => "iand",
        I::Land => "land",
        I::Ior => "ior",
        I::Lor => "lor",
        I::Ixor => "ixor",
        I::Lxor => "lxor",
        I::Iinc { .. } | I::IincW { .. } => "iinc",
        I::I2l => "i2l",
        I::I2f => "i2f",
        I::I2d => "i2d",
        I::L2i => "l2i",
        I::L2f => "l2f",
        I::L2d => "l2d",
        I::F2i => "f2i",
        I::F2l => "f2l",
        I::F2d => "f2d",
        I::D2i => "d2i",
        I::D2l => "d2l",
        I::D2f => "d2f",
        I::I2b => "i2b",
        I::I2c => "i2c",
        I::I2s => "i2s",
        I::Lcmp => "lcmp",
        I::Fcmpl => "fcmpl",
        I::Fcmpg => "fcmpg",
        I::Dcmpl => "dcmpl",
        I::Dcmpg => "dcmpg",
        I::Ifeq(_) | I::Ifne(_) | I::Iflt(_) | I::Ifge(_) | I::Ifgt(_) | I::Ifle(_) => "if_<cond>",
        I::IfIcmpeq(_)
        | I::IfIcmpne(_)
        | I::IfIcmplt(_)
        | I::IfIcmpge(_)
        | I::IfIcmpgt(_)
        | I::IfIcmple(_) => "if_icmp<cond>",
        I::IfAcmpeq(_) | I::IfAcmpne(_) => "if_acmp<cond>",
        I::Goto(_) | I::GotoW(_) => "goto",
        I::Jsr(_) | I::JsrW(_) => "jsr",
        I::Ret(_) | I::RetW(_) => "ret",
        I::Tableswitch { .. } => "tableswitch",
        I::Lookupswitch { .. } => "lookupswitch",
        I::Ireturn => "ireturn",
        I::Lreturn => "lreturn",
        I::Freturn => "freturn",
        I::Dreturn => "dreturn",
        I::Areturn => "areturn",
        I::Return => "return",
        I::Getstatic(_) => "getstatic",
        I::Putstatic(_) => "putstatic",
        I::Getfield(_) => "getfield",
        I::Putfield(_) => "putfield",
        I::Invokevirtual(_) => "invokevirtual",
        I::Invokespecial(_) => "invokespecial",
        I::Invokestatic(_) => "invokestatic",
        I::Invokeinterface { .. } => "invokeinterface",
        I::Invokedynamic(_) => "invokedynamic",
        I::New(_) => "new",
        I::Newarray(_) => "newarray",
        I::Anewarray(_) => "anewarray",
        I::Arraylength => "arraylength",
        I::Athrow => "athrow",
        I::Checkcast(_) => "checkcast",
        I::Instanceof(_) => "instanceof",
        I::Monitorenter => "monitorenter",
        I::Monitorexit => "monitorexit",
        I::Multianewarray { .. } => "multianewarray",
        I::Ifnull(_) | I::Ifnonnull(_) => "ifnull/nonnull",
    }
}

/// Execute a static method by name within a loaded class context.
///
/// Supports `invokestatic` calls between methods in the same class.
///
/// # Errors
/// Returns [`VmError`] on execution faults or if `method_name`/`descriptor`
/// are not found in `ctx`.
///
/// # Panics
/// Panics on internal invariant violations, such as a `Class` CP entry whose
/// name index refers to a non-`Utf8` entry (indicates a malformed class file).
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::manual_let_else,
    clippy::single_match,
    clippy::single_match_else,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::used_underscore_binding
)]
pub fn execute_class(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>> {
    // Fast path: if a native handler is registered for this class/method/descriptor,
    // dispatch it directly without requiring a ClassContext in the registry.
    // This handles both Simple natives and Callback natives at the top-level call site.
    match registry
        .natives_mut()
        .get_kind(class_name, method_name, descriptor)
    {
        Some(HandlerKind::Simple(h)) => {
            let mut native_control = NativeControl::default();
            let result = h(args, heap, stdout, &mut native_control)?;
            if native_control.take().is_some() {
                return Err(VmError::Unimplemented {
                    mnemonic: "thread action requires execute_class_to_completion",
                });
            }
            return Ok(result);
        }
        Some(HandlerKind::Callback(h)) => {
            let mut native_control = NativeControl::default();
            let result = {
                let mut callback_ops = InterpreterCallbackOps { registry, loader };
                h(args, heap, stdout, &mut native_control, &mut callback_ops)?
            };
            if native_control.take().is_some() {
                return Err(VmError::Unimplemented {
                    mnemonic: "thread action requires execute_class_to_completion",
                });
            }
            return Ok(result);
        }
        None => {}
    }

    let mut state = prepare_execution_state(
        registry,
        loader,
        heap,
        stdout,
        class_name,
        method_name,
        descriptor,
        args,
    )?;
    match run_execution(&mut state, registry, loader, heap, stdout, true)? {
        ExecutionOutcome::Returned(result) => Ok(result),
        ExecutionOutcome::ThreadAction(_) => Err(VmError::Unimplemented {
            mnemonic: "thread action requires execute_class_to_completion",
        }),
    }
}

#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::manual_let_else,
    clippy::single_match,
    clippy::single_match_else,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::used_underscore_binding
)]
fn run_execution(
    state: &mut ExecutionState,
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    gc_allowed: bool,
) -> VmResult<ExecutionOutcome> {
    let ExecutionState {
        current_class,
        method_idx,
        pc_to_idx,
        instructions,
        frame,
        call_stack,
        frame_pool,
        dispatch_cache,
        idx,
        string_intern,
        #[cfg(feature = "telemetry")]
        current_method,
    } = state;

    loop {
        let (pc, instr) = {
            let Some(&(pc, ref instr)) = instructions.get(*idx) else {
                return Err(VmError::FellOffEnd);
            };
            (pc, instr.clone())
        };

        macro_rules! jump {
            ($offset:expr) => {{
                let target = (pc as i64).wrapping_add(i64::from($offset)) as usize;
                *idx = *pc_to_idx
                    .get(&target)
                    .ok_or(VmError::InvalidBranchTarget { pc: target })?;
                continue;
            }};
        }

        // Helper macro for return instructions: pop call stack or return to Rust.
        macro_rules! do_return {
            ($val:expr) => {{
                match call_stack.pop() {
                    None => return Ok(ExecutionOutcome::Returned($val)),
                    Some(caller) => {
                        let ret_val = $val;
                        // Recycle callee frame buffers before overwriting `frame`.
                        let old = std::mem::replace(frame, caller.frame);
                        let (l, s) = old.into_pool_bufs();
                        frame_pool.release(l, s);
                        *method_idx = caller.method_idx;
                        *pc_to_idx = caller.pc_to_idx;
                        *idx = caller.resume_idx;
                        *current_class = caller.class_name;
                        *instructions = std::sync::Arc::clone(
                            &registry.get(&current_class)?.methods[*method_idx].instructions,
                        );
                        #[cfg(feature = "telemetry")]
                        {
                            *current_method = registry
                                .get(&current_class)
                                .map(|c| {
                                    c.methods
                                        .get(*method_idx)
                                        .map(|m| m.name.clone())
                                        .unwrap_or_default()
                                })
                                .unwrap_or_default();
                        }
                        if let Some(v) = ret_val {
                            frame.push(v)?;
                        }
                        continue;
                    }
                }
            }};
        }

        macro_rules! propagate_java_exception {
            ($exc_class_name:expr, $exception_ref:expr, $throw_pc:expr) => {{
                let exc_class_name = $exc_class_name;
                let exception_ref = $exception_ref;

                #[cfg(feature = "telemetry")]
                let _telem_exc_event_idx = registry.telemetry.exception_flow.record_throw(
                    &exc_class_name,
                    &current_class,
                    &current_method,
                    $throw_pc,
                );

                // Clone the exception table to release the borrow on registry,
                // so find_exception_handler can use &mut registry for hierarchy checks.
                let exc_table = registry.get(&current_class)?.methods[*method_idx]
                    .exception_table
                    .iter()
                    .map(|e| ExceptionEntry {
                        start_pc: e.start_pc,
                        end_pc: e.end_pc,
                        handler_pc: e.handler_pc,
                        catch_type: e.catch_type.clone(),
                    })
                    .collect::<Vec<_>>();
                let handler = find_exception_handler(
                    &exc_table,
                    $throw_pc,
                    &exc_class_name,
                    registry,
                    loader,
                );
                if let Some(handler_pc) = handler {
                    #[cfg(feature = "telemetry")]
                    registry.telemetry.exception_flow.record_catch(
                        _telem_exc_event_idx,
                        &current_class,
                        &current_method,
                        handler_pc as usize,
                    );
                    frame.clear_stack();
                    frame.push(Slot::Reference(Some(exception_ref)))?;
                    *idx = *pc_to_idx.get(&(handler_pc as usize)).ok_or(
                        VmError::InvalidBranchTarget {
                            pc: handler_pc as usize,
                        },
                    )?;
                    continue;
                }

                loop {
                    match call_stack.pop() {
                        None => {
                            return Err(VmError::JavaException {
                                class_name: exc_class_name,
                            });
                        }
                        Some(caller) => {
                            // Recycle callee frame buffers before overwriting `frame` —
                            // mirrors the do_return! pattern to avoid a pool leak.
                            let old = std::mem::replace(frame, caller.frame);
                            let (l, s) = old.into_pool_bufs();
                            frame_pool.release(l, s);
                            *method_idx = caller.method_idx;
                            *pc_to_idx = caller.pc_to_idx;
                            *current_class = caller.class_name;
                            *instructions = std::sync::Arc::clone(
                                &registry.get(&current_class)?.methods[*method_idx].instructions,
                            );
                            #[cfg(feature = "telemetry")]
                            {
                                *current_method = registry
                                    .get(&current_class)
                                    .map(|c| {
                                        c.methods
                                            .get(*method_idx)
                                            .map(|m| m.name.clone())
                                            .unwrap_or_default()
                                    })
                                    .unwrap_or_default();
                            }

                            let (caller_exc_table, caller_pc) = {
                                let ctx = registry.get(&current_class)?;
                                let cpc = if caller.resume_idx > 0 {
                                    ctx.methods[*method_idx].instructions[caller.resume_idx - 1].0
                                } else {
                                    0
                                };
                                let tbl = ctx.methods[*method_idx]
                                    .exception_table
                                    .iter()
                                    .map(|e| ExceptionEntry {
                                        start_pc: e.start_pc,
                                        end_pc: e.end_pc,
                                        handler_pc: e.handler_pc,
                                        catch_type: e.catch_type.clone(),
                                    })
                                    .collect::<Vec<_>>();
                                (tbl, cpc)
                            };
                            let handler = find_exception_handler(
                                &caller_exc_table,
                                caller_pc,
                                &exc_class_name,
                                registry,
                                loader,
                            );

                            if let Some(handler_pc) = handler {
                                #[cfg(feature = "telemetry")]
                                registry.telemetry.exception_flow.record_catch(
                                    _telem_exc_event_idx,
                                    &current_class,
                                    &current_method,
                                    handler_pc as usize,
                                );
                                frame.clear_stack();
                                frame.push(Slot::Reference(Some(exception_ref)))?;
                                *idx = *pc_to_idx.get(&(handler_pc as usize)).ok_or(
                                    VmError::InvalidBranchTarget {
                                        pc: handler_pc as usize,
                                    },
                                )?;
                                break;
                            }
                        }
                    }
                }
                continue;
            }};
        }

        // Telemetry: capture opcode name and start time before dispatch.
        // Arms that use `continue` (branches, invokes) will skip the post-match
        // recording for that iteration — timing is approximate for those opcodes.
        #[cfg(feature = "telemetry")]
        #[allow(clippy::used_underscore_binding)]
        let (_telem_name, _telem_pc, _telem_start) = {
            let name = instr_name(&instr);
            let pc_val = pc;
            (name, pc_val, std::time::Instant::now())
        };

        match &instr {
            // ---- invokestatic ----
            Instruction::Invokestatic(cp_idx) => {
                if let Some(&(ref cached_cls, cached_idx, cached_ac)) = dispatch_cache
                    .get(current_class.as_str())
                    .and_then(|m| m.get(&cp_idx.0))
                {
                    // Fast path: cache hit — skip CP walk and method search.
                    let (callee_pc_to_idx, callee_frame) = {
                        let ctx = registry.get(cached_cls)?;
                        let max_locals = usize::from(ctx.methods[cached_idx].max_locals);
                        let max_stack = usize::from(ctx.methods[cached_idx].max_stack);
                        let pci = std::sync::Arc::clone(&ctx.methods[cached_idx].pc_to_idx);
                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                        locals_buf.resize(max_locals, Slot::Int(0));
                        if cached_ac > max_locals {
                            return Err(VmError::LocalOutOfBounds {
                                index: cached_ac,
                                max_locals,
                            });
                        }
                        for i in (0..cached_ac).rev() {
                            locals_buf[i] = frame.pop()?;
                        }
                        let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                        (pci, f)
                    };
                    let callee_class = cached_cls.clone();
                    let callee_idx = cached_idx;
                    activate_method_state(
                        frame,
                        method_idx,
                        pc_to_idx,
                        instructions,
                        current_class,
                        call_stack,
                        registry,
                        callee_class,
                        callee_idx,
                        callee_pc_to_idx,
                        callee_frame,
                        *idx + 1,
                        #[cfg(feature = "telemetry")]
                        current_method,
                    )?;
                    *idx = 0;
                    continue;
                }
                // Slow path: full CP resolution + method search.
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                registry.ensure_loaded(&callee_class, loader)?;
                ensure_initialized(registry, loader, heap, stdout, &callee_class, current_class)?;
                let callee_idx = {
                    let ctx = registry.get(&callee_class)?;
                    ctx.methods
                        .iter()
                        .position(|m| m.name == callee_name && m.descriptor == callee_desc)
                };
                match callee_idx {
                    Some(callee_idx) => {
                        let arg_count = parse_arg_count(&callee_desc);
                        dispatch_cache
                            .entry(current_class.clone())
                            .or_default()
                            .insert(cp_idx.0, (callee_class.clone(), callee_idx, arg_count));
                        let (callee_pc_to_idx, callee_frame) = {
                            let ctx = registry.get(&callee_class)?;
                            let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                            let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                            let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                            let (mut locals_buf, stack_buf) = frame_pool.acquire();
                            locals_buf.resize(max_locals, Slot::Int(0));
                            if arg_count > max_locals {
                                return Err(VmError::LocalOutOfBounds {
                                    index: arg_count,
                                    max_locals,
                                });
                            }
                            for i in (0..arg_count).rev() {
                                locals_buf[i] = frame.pop()?;
                            }
                            let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                            (pci, f)
                        };
                        activate_method_state(
                            frame,
                            method_idx,
                            pc_to_idx,
                            instructions,
                            current_class,
                            call_stack,
                            registry,
                            callee_class,
                            callee_idx,
                            callee_pc_to_idx,
                            callee_frame,
                            *idx + 1,
                            #[cfg(feature = "telemetry")]
                            current_method,
                        )?;
                        *idx = 0;
                        continue;
                    }
                    None => {
                        // Check native registry before erroring.
                        let handler_kind = registry.natives_mut().get_kind(
                            &callee_class,
                            &callee_name,
                            &callee_desc,
                        );
                        match handler_kind {
                            Some(HandlerKind::Simple(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut native_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }

                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = NativeControl::default();
                                let result =
                                    handler(&native_args, heap, stdout, &mut native_control);
                                #[cfg(feature = "telemetry")]
                                registry.telemetry.native_boundary.record_call(
                                    &callee_class,
                                    &callee_name,
                                    _native_start.elapsed().as_nanos() as u64,
                                    result.is_err(),
                                );
                                let result = match result {
                                    Ok(result) => result,
                                    Err(VmError::JavaException { class_name }) => {
                                        let exception_ref = materialize_java_exception_object(
                                            registry,
                                            loader,
                                            heap,
                                            &class_name,
                                        )?;
                                        propagate_java_exception!(class_name, exception_ref, pc);
                                    }
                                    Err(err) => return Err(err),
                                };
                                if let Some(outcome) =
                                    finish_native_call(&mut native_control, frame, idx, result)?
                                {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            Some(HandlerKind::Callback(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut native_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = NativeControl::default();
                                let result = {
                                    let mut callback_ops =
                                        InterpreterCallbackOps { registry, loader };
                                    handler(
                                        &native_args,
                                        heap,
                                        stdout,
                                        &mut native_control,
                                        &mut callback_ops,
                                    )
                                };
                                #[cfg(feature = "telemetry")]
                                registry.telemetry.native_boundary.record_call(
                                    &callee_class,
                                    &callee_name,
                                    _native_start.elapsed().as_nanos() as u64,
                                    result.is_err(),
                                );
                                let result = match result {
                                    Ok(result) => result,
                                    Err(VmError::JavaException { class_name }) => {
                                        let exception_ref = materialize_java_exception_object(
                                            registry,
                                            loader,
                                            heap,
                                            &class_name,
                                        )?;
                                        propagate_java_exception!(class_name, exception_ref, pc);
                                    }
                                    Err(err) => return Err(err),
                                };
                                if let Some(outcome) =
                                    finish_native_call(&mut native_control, frame, idx, result)?
                                {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            None => {
                                return Err(VmError::MethodNotFound {
                                    name: callee_name,
                                    descriptor: callee_desc,
                                });
                            }
                        }
                    }
                }
            }

            // ---- returns ----
            Instruction::Return => do_return!(None),
            Instruction::Ireturn => {
                let v = frame.pop_int()?;
                do_return!(Some(Slot::Int(v)));
            }
            Instruction::Lreturn => {
                let v = frame.pop_long()?;
                do_return!(Some(Slot::Long(v)));
            }
            Instruction::Freturn => {
                let v = frame.pop_float()?;
                do_return!(Some(Slot::Float(v)));
            }
            Instruction::Dreturn => {
                let v = frame.pop_double()?;
                do_return!(Some(Slot::Double(v)));
            }

            // ---- all other instructions: same as execute() ----
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
            Instruction::Ldc(raw_idx) => {
                let cp_idx = usize::from(*raw_idx);
                let string_info = {
                    let ctx = registry.get(current_class)?;
                    if let Some(CpEntry::String { string_index }) =
                        ctx.constant_pool.get(cp_idx).and_then(|e| e.as_ref())
                    {
                        let si = string_index.0 as usize;
                        let s = match ctx.constant_pool.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        Some(s)
                    } else {
                        None
                    }
                };
                if let Some(s) = string_info {
                    let r = if let Some(&cached) = string_intern.get(&cp_idx) {
                        cached
                    } else {
                        let r = heap.allocate_string(s);
                        string_intern.insert(cp_idx, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    // Check for Class constant
                    let class_info = {
                        let ctx = registry.get(current_class)?;
                        if let Some(CpEntry::Class { name_index }) =
                            ctx.constant_pool.get(cp_idx).and_then(|e| e.as_ref())
                        {
                            match ctx
                                .constant_pool
                                .get(name_index.0 as usize)
                                .and_then(|e| e.as_ref())
                            {
                                Some(CpEntry::Utf8(s)) => Some(s.clone()),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    };
                    if let Some(class_name) = class_info {
                        // Intern class literals using offset key to avoid collision with String interning
                        let intern_key = cp_idx + 100_000;
                        let r = if let Some(&cached) = string_intern.get(&intern_key) {
                            cached
                        } else {
                            let r = heap.allocate("java/lang/Class".to_string(), 0);
                            heap.get_mut(r).unwrap().string_value = Some(class_name);
                            string_intern.insert(intern_key, r);
                            r
                        };
                        frame.push(Slot::Reference(Some(r)))?;
                    } else {
                        let ctx = registry.get(current_class)?;
                        ldc_push(frame, &ctx.constant_pool, cp_idx)?;
                    }
                }
            }
            Instruction::LdcW(cp_idx) | Instruction::Ldc2W(cp_idx) => {
                let idx_val = usize::from(cp_idx.0);
                let string_info = {
                    let ctx = registry.get(current_class)?;
                    if let Some(CpEntry::String { string_index }) =
                        ctx.constant_pool.get(idx_val).and_then(|e| e.as_ref())
                    {
                        let si = string_index.0 as usize;
                        let s = match ctx.constant_pool.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        Some(s)
                    } else {
                        None
                    }
                };
                if let Some(s) = string_info {
                    let r = if let Some(&cached) = string_intern.get(&idx_val) {
                        cached
                    } else {
                        let r = heap.allocate_string(s);
                        string_intern.insert(idx_val, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    // Check for Class constant
                    let class_info = {
                        let ctx = registry.get(current_class)?;
                        if let Some(CpEntry::Class { name_index }) =
                            ctx.constant_pool.get(idx_val).and_then(|e| e.as_ref())
                        {
                            match ctx
                                .constant_pool
                                .get(name_index.0 as usize)
                                .and_then(|e| e.as_ref())
                            {
                                Some(CpEntry::Utf8(s)) => Some(s.clone()),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    };
                    if let Some(class_name) = class_info {
                        // Intern class literals using offset key to avoid collision with String interning
                        let intern_key = idx_val + 100_000;
                        let r = if let Some(&cached) = string_intern.get(&intern_key) {
                            cached
                        } else {
                            let r = heap.allocate("java/lang/Class".to_string(), 0);
                            heap.get_mut(r).unwrap().string_value = Some(class_name);
                            string_intern.insert(intern_key, r);
                            r
                        };
                        frame.push(Slot::Reference(Some(r)))?;
                    } else {
                        let ctx = registry.get(current_class)?;
                        ldc_push(frame, &ctx.constant_pool, idx_val)?;
                    }
                }
            }
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
            Instruction::Pop => {
                frame.pop()?;
            }
            Instruction::Pop2 => {
                frame.pop()?;
                frame.pop()?;
            }
            Instruction::Dup => {
                let v = frame.pop()?;
                frame.push(v)?;
                frame.push(v)?;
            }
            Instruction::DupX1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::DupX2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                let v4 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v4)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Swap => {
                let a = frame.pop()?;
                let b = frame.pop()?;
                frame.push(a)?;
                frame.push(b)?;
            }
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
                    1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }
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
                    1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }
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
                frame.push(Slot::Int(i32::from(v as i8)))?;
            }
            Instruction::I2c => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as u16)))?;
            }
            Instruction::I2s => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as i16)))?;
            }
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
            Instruction::IfAcmpeq(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfAcmpne(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a != b {
                    jump!(*offset);
                }
            }

            // ---- Object allocation ----
            Instruction::New(cp_idx) => {
                let target_class = {
                    let ctx = registry.get(current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                registry.ensure_loaded(&target_class, loader)?;
                ensure_initialized(registry, loader, heap, stdout, &target_class, current_class)?;
                // Walk the super chain to sum all instance field counts
                // (e.g. Enum has 2 fields inherited by every enum subclass).
                let field_count = total_instance_field_count(registry, &target_class);
                #[cfg(feature = "telemetry")]
                registry.telemetry.object_lineage.record(
                    current_class,
                    current_method,
                    pc,
                    &target_class,
                );
                let r = heap.allocate(target_class.clone(), field_count);
                // Set reference/long/float/double fields to their JVM-spec defaults.
                // heap.allocate initialises everything to Int(0), which is wrong
                // for reference-typed fields (should be Reference(None)).
                init_object_fields(registry, heap, r, &target_class);
                frame.push(Slot::Reference(Some(r)))?;
                if gc_allowed && heap.should_gc() {
                    let roots = gather_roots(frame, call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(frame, call_stack, registry, heap);
                }
            }

            // ---- Field access ----
            Instruction::Getfield(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let r = frame.pop_ref()?;
                registry.ensure_loaded(&target_class, loader)?;
                let fidx = field_slot_idx(registry, &target_class, &field_name)?;
                let val = heap.get(r)?.fields[fidx];
                frame.push(val)?;
            }
            Instruction::Putfield(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let val = frame.pop()?;
                let r = frame.pop_ref()?;
                registry.ensure_loaded(&target_class, loader)?;
                let fidx = field_slot_idx(registry, &target_class, &field_name)?;
                heap.write_field(r, fidx, val)?;
            }
            Instruction::Getstatic(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                registry.ensure_loaded(&target_class, loader)?;
                ensure_initialized(registry, loader, heap, stdout, &target_class, current_class)?;
                let sidx = static_field_idx(registry.get(&target_class)?, &field_name)?;
                let val = registry.get(&target_class)?.static_fields[sidx];
                frame.push(val)?;
            }
            Instruction::Putstatic(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let val = frame.pop()?;
                registry.ensure_loaded(&target_class, loader)?;
                ensure_initialized(registry, loader, heap, stdout, &target_class, current_class)?;
                let sidx = static_field_idx(registry.get(&target_class)?, &field_name)?;
                registry.get_mut(&target_class)?.static_fields[sidx] = val;
            }

            // ---- Instance method dispatch ----
            //
            // invokespecial and invokevirtual: resolve class+name+descriptor from
            // the Methodref.  Dispatch cross-class via registry; unloadable
            // classes (e.g. java/lang/Object) fall back to no-op.
            Instruction::Invokespecial(cp_idx) | Instruction::Invokevirtual(cp_idx) => {
                // Fast path: cache hit for invokespecial (static dispatch — safe to cache).
                if matches!(instr, Instruction::Invokespecial(_))
                    && let Some(&(ref cached_cls, cached_idx, cached_ac)) = dispatch_cache
                        .get(current_class.as_str())
                        .and_then(|m| m.get(&cp_idx.0))
                {
                    let (callee_pc_to_idx, callee_frame) = {
                        let ctx = registry.get(cached_cls)?;
                        let max_locals = usize::from(ctx.methods[cached_idx].max_locals);
                        let max_stack = usize::from(ctx.methods[cached_idx].max_stack);
                        let pci = std::sync::Arc::clone(&ctx.methods[cached_idx].pc_to_idx);
                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                        locals_buf.resize(max_locals, Slot::Int(0));
                        if cached_ac + 1 > max_locals {
                            return Err(VmError::LocalOutOfBounds {
                                index: cached_ac + 1,
                                max_locals,
                            });
                        }
                        for i in (1..=cached_ac).rev() {
                            locals_buf[i] = frame.pop()?;
                        }
                        locals_buf[0] = frame.pop()?; // `this`
                        let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                        (pci, f)
                    };
                    let dispatch_class = cached_cls.clone();
                    let callee_idx = cached_idx;
                    activate_method_state(
                        frame,
                        method_idx,
                        pc_to_idx,
                        instructions,
                        current_class,
                        call_stack,
                        registry,
                        dispatch_class,
                        callee_idx,
                        callee_pc_to_idx,
                        callee_frame,
                        *idx + 1,
                        #[cfg(feature = "telemetry")]
                        current_method,
                    )?;
                    *idx = 0;
                    continue;
                }
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                // <clinit> (static initialiser) is not supported yet — skip silently.
                if callee_name == "<clinit>" {
                    *idx += 1;
                    continue;
                }
                // Attempt to load the target class; soft-fail for unloadable.
                let class_was_loaded = registry.ensure_loaded(&callee_class, loader)?;
                let resolved = if class_was_loaded {
                    resolve_method_in_hierarchy(
                        registry,
                        loader,
                        &callee_class,
                        &callee_name,
                        &callee_desc,
                    )
                } else {
                    None
                };
                let (dispatch_class, callee_idx) = match resolved {
                    Some((cls, i)) => (cls, i),
                    None => {
                        // Check lambda dispatch before native fallback.
                        let arg_count = parse_arg_count(&callee_desc);
                        let stack_len = frame.stack_len();
                        if stack_len > arg_count {
                            let this_pos = stack_len - arg_count - 1;
                            let actual_class_opt =
                                if let Ok(Slot::Reference(Some(r))) = frame.peek_at(this_pos) {
                                    heap.get(r).ok().map(|o| o.class_name.clone())
                                } else {
                                    None
                                };

                            if let Some(ref actual_class) = actual_class_opt
                                && let Some(lambda_info) =
                                    registry.get_lambda(actual_class).cloned()
                                && callee_name == lambda_info.sam_method
                            {
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut sam_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    sam_args[i] = frame.pop()?;
                                }
                                let this_slot = frame.pop()?;
                                let this_ref = match &this_slot {
                                    Slot::Reference(Some(r)) => *r,
                                    _ => return Err(VmError::NullPointerException),
                                };

                                let obj = heap.get(this_ref)?;
                                let mut impl_args: Vec<Slot> = Vec::new();
                                for i in 0..lambda_info.captured_count {
                                    impl_args.push(obj.fields[i]);
                                }
                                impl_args.extend(sam_args.iter().copied());

                                let _ = registry.ensure_loaded(&lambda_info.impl_class, loader);

                                let resolved = resolve_method_in_hierarchy(
                                    registry,
                                    loader,
                                    &lambda_info.impl_class,
                                    &lambda_info.impl_method,
                                    &lambda_info.impl_desc,
                                );
                                if let Some((dispatch_class, impl_idx)) = resolved {
                                    let (callee_pc_to_idx, callee_frame) = {
                                        let ctx = registry.get(&dispatch_class)?;
                                        let max_locals =
                                            usize::from(ctx.methods[impl_idx].max_locals);
                                        let max_stack =
                                            usize::from(ctx.methods[impl_idx].max_stack);
                                        let pci =
                                            std::sync::Arc::clone(&ctx.methods[impl_idx].pc_to_idx);
                                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                        locals_buf.resize(max_locals, Slot::Int(0));
                                        for (i, slot) in impl_args.into_iter().enumerate() {
                                            locals_buf[i] = slot;
                                        }
                                        let f =
                                            Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                                        (pci, f)
                                    };
                                    activate_method_state(
                                        frame,
                                        method_idx,
                                        pc_to_idx,
                                        instructions,
                                        current_class,
                                        call_stack,
                                        registry,
                                        dispatch_class,
                                        impl_idx,
                                        callee_pc_to_idx,
                                        callee_frame,
                                        *idx + 1,
                                        #[cfg(feature = "telemetry")]
                                        current_method,
                                    )?;
                                    *idx = 0;
                                    continue;
                                }
                            }
                        }
                        // Check native registry, walking the super chain.
                        let native_handler_kind = {
                            // For invokevirtual: first try the actual runtime class of
                            // `this` so that subclass natives (e.g. duke/net/SocketInputStream
                            // registered under read:()I) are found even when the call-site
                            // declares the abstract base (java/io/InputStream).
                            let virtual_start: Option<String> =
                                if matches!(instr, Instruction::Invokevirtual(_)) {
                                    let arg_count = parse_arg_count(&callee_desc);
                                    let stack_len = frame.stack_len();
                                    if stack_len > arg_count {
                                        let this_pos = stack_len - arg_count - 1;
                                        if let Ok(Slot::Reference(Some(r))) =
                                            frame.peek_at(this_pos)
                                        {
                                            heap.get(r).ok().map(|o| o.class_name.clone())
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };

                            let mut found = None;

                            // Walk from runtime class first (invokevirtual virtual dispatch).
                            if let Some(ref runtime_class) = virtual_start {
                                let mut sc: Option<String> = Some(runtime_class.clone());
                                while let Some(ref s) = sc {
                                    if let Some(h) = registry.natives_mut().get_kind(
                                        s,
                                        &callee_name,
                                        &callee_desc,
                                    ) {
                                        found = Some(h);
                                        break;
                                    }
                                    sc = registry.get(s).ok().and_then(|c| c.super_class.clone());
                                }
                            }

                            // Fall back to declared (callee_class) super chain.
                            if found.is_none() {
                                found = registry.natives_mut().get_kind(
                                    &callee_class,
                                    &callee_name,
                                    &callee_desc,
                                );
                            }
                            if found.is_none() {
                                // Walk super chain for native lookup (e.g. Enum.ordinal
                                // called via SimpleEnum$Color.ordinal).
                                let start = if callee_class.starts_with('[') {
                                    // Arrays inherit from Object.
                                    Some("java/lang/Object".to_string())
                                } else {
                                    registry
                                        .get(&callee_class)
                                        .ok()
                                        .and_then(|c| c.super_class.clone())
                                };
                                let mut sc = start;
                                while let Some(ref s) = sc {
                                    if let Some(h) = registry.natives_mut().get_kind(
                                        s,
                                        &callee_name,
                                        &callee_desc,
                                    ) {
                                        found = Some(h);
                                        break;
                                    }
                                    sc = registry.get(s).ok().and_then(|c| c.super_class.clone());
                                }
                            }
                            found
                        };
                        match native_handler_kind {
                            Some(HandlerKind::Simple(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut native_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }

                                let this_slot = frame.pop()?; // pop `this`
                                native_args.insert(0, this_slot);
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = NativeControl::default();
                                let result =
                                    handler(&native_args, heap, stdout, &mut native_control);
                                #[cfg(feature = "telemetry")]
                                {
                                    registry.telemetry.native_boundary.record_call(
                                        &callee_class,
                                        &callee_name,
                                        _native_start.elapsed().as_nanos() as u64,
                                        result.is_err(),
                                    );
                                    if matches!(instr, Instruction::Invokevirtual(_)) {
                                        // Native methods do not perform a bytecode hierarchy
                                        // walk — hierarchy_walk is always false here.
                                        registry.telemetry.dispatch_resolution.record(
                                            current_class,
                                            cp_idx.0,
                                            &callee_class,
                                            false,
                                        );
                                    }
                                }
                                let result = match result {
                                    Ok(result) => result,
                                    Err(VmError::JavaException { class_name }) => {
                                        let exception_ref = materialize_java_exception_object(
                                            registry,
                                            loader,
                                            heap,
                                            &class_name,
                                        )?;
                                        propagate_java_exception!(class_name, exception_ref, pc);
                                    }
                                    Err(err) => return Err(err),
                                };
                                if let Some(outcome) =
                                    finish_native_call(&mut native_control, frame, idx, result)?
                                {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            Some(HandlerKind::Callback(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut native_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }
                                let this_slot = frame.pop()?; // pop `this`
                                native_args.insert(0, this_slot);
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut native_control = NativeControl::default();
                                let result = {
                                    let mut callback_ops =
                                        InterpreterCallbackOps { registry, loader };
                                    handler(
                                        &native_args,
                                        heap,
                                        stdout,
                                        &mut native_control,
                                        &mut callback_ops,
                                    )
                                };
                                #[cfg(feature = "telemetry")]
                                {
                                    registry.telemetry.native_boundary.record_call(
                                        &callee_class,
                                        &callee_name,
                                        _native_start.elapsed().as_nanos() as u64,
                                        result.is_err(),
                                    );
                                    if matches!(instr, Instruction::Invokevirtual(_)) {
                                        registry.telemetry.dispatch_resolution.record(
                                            current_class,
                                            cp_idx.0,
                                            &callee_class,
                                            false,
                                        );
                                    }
                                }
                                let result = match result {
                                    Ok(result) => result,
                                    Err(VmError::JavaException { class_name }) => {
                                        let exception_ref = materialize_java_exception_object(
                                            registry,
                                            loader,
                                            heap,
                                            &class_name,
                                        )?;
                                        propagate_java_exception!(class_name, exception_ref, pc);
                                    }
                                    Err(err) => return Err(err),
                                };
                                if let Some(outcome) =
                                    finish_native_call(&mut native_control, frame, idx, result)?
                                {
                                    return Ok(outcome);
                                }
                                continue;
                            }
                            None => {
                                // Unloadable or missing — pop args + this and continue.
                                let arg_count = parse_arg_count(&callee_desc);
                                for _ in 0..arg_count {
                                    frame.pop()?;
                                }
                                frame.pop()?; // pop `this`
                                *idx += 1;
                                continue;
                            }
                        }
                    }
                };
                let arg_count = parse_arg_count(&callee_desc);
                // Populate dispatch cache for invokespecial (static dispatch — result is stable).
                if matches!(instr, Instruction::Invokespecial(_)) {
                    dispatch_cache
                        .entry(current_class.clone())
                        .or_default()
                        .insert(cp_idx.0, (dispatch_class.clone(), callee_idx, arg_count));
                }
                #[cfg(feature = "telemetry")]
                if matches!(instr, Instruction::Invokevirtual(_)) {
                    registry.telemetry.dispatch_resolution.record(
                        current_class,
                        cp_idx.0,
                        &dispatch_class,
                        dispatch_class != callee_class,
                    );
                }
                let (callee_pc_to_idx, callee_frame) = {
                    let ctx = registry.get(&dispatch_class)?;
                    let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                    let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                    let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                    let (mut locals_buf, stack_buf) = frame_pool.acquire();
                    locals_buf.resize(max_locals, Slot::Int(0));
                    if arg_count + 1 > max_locals {
                        return Err(VmError::LocalOutOfBounds {
                            index: arg_count + 1,
                            max_locals,
                        });
                    }
                    // Pop args into locals[1..=arg_count] in reverse (stack top = last arg).
                    for i in (1..=arg_count).rev() {
                        locals_buf[i] = frame.pop()?;
                    }
                    locals_buf[0] = frame.pop()?; // `this`
                    let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                    (pci, f)
                };
                activate_method_state(
                    frame,
                    method_idx,
                    pc_to_idx,
                    instructions,
                    current_class,
                    call_stack,
                    registry,
                    dispatch_class,
                    callee_idx,
                    callee_pc_to_idx,
                    callee_frame,
                    *idx + 1,
                    #[cfg(feature = "telemetry")]
                    current_method,
                )?;
                *idx = 0;
                continue;
            }

            // ---- Reference return ----
            Instruction::Areturn => {
                let v = frame.pop()?;
                do_return!(Some(v));
            }

            // ----------------------------------------------------------------
            // Array allocation
            // ----------------------------------------------------------------
            Instruction::Newarray(array_type) => {
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let class_name = match array_type {
                    ArrayType::Boolean => "[Z",
                    ArrayType::Char => "[C",
                    ArrayType::Float => "[F",
                    ArrayType::Double => "[D",
                    ArrayType::Byte => "[B",
                    ArrayType::Short => "[S",
                    ArrayType::Int => "[I",
                    ArrayType::Long => "[J",
                };
                let r = heap.allocate(class_name.to_string(), count as usize);
                // Fix element types for non-int primitive arrays.
                match array_type {
                    ArrayType::Long => {
                        let obj = heap.get_mut(r)?;
                        for slot in &mut obj.fields {
                            *slot = Slot::Long(0);
                        }
                    }
                    ArrayType::Float => {
                        let obj = heap.get_mut(r)?;
                        for slot in &mut obj.fields {
                            *slot = Slot::Float(0.0);
                        }
                    }
                    ArrayType::Double => {
                        let obj = heap.get_mut(r)?;
                        for slot in &mut obj.fields {
                            *slot = Slot::Double(0.0);
                        }
                    }
                    _ => {} // Int/Boolean/Byte/Char/Short default to Slot::Int(0)
                }
                frame.push(Slot::Reference(Some(r)))?;
                if gc_allowed && heap.should_gc() {
                    let roots = gather_roots(frame, call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(frame, call_stack, registry, heap);
                }
            }
            Instruction::Anewarray(cp_idx) => {
                let element_type = {
                    let ctx = registry.get(current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let array_type = format!("[L{element_type};");
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let r = heap.allocate(array_type, count as usize);
                // Fix elements to Reference(None).
                let obj = heap.get_mut(r)?;
                for slot in &mut obj.fields {
                    *slot = Slot::Reference(None);
                }
                frame.push(Slot::Reference(Some(r)))?;
                if gc_allowed && heap.should_gc() {
                    let roots = gather_roots(frame, call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(frame, call_stack, registry, heap);
                }
            }
            Instruction::Arraylength => {
                let r = frame.pop_ref()?;
                let len = heap.get(r)?.fields.len();
                frame.push(Slot::Int(len as i32))?;
            }

            // ---- Int array ----
            Instruction::Iaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_int()?
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Iastore => {
                let val = frame.pop_int()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Long array ----
            Instruction::Laload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_long()?
                };
                frame.push(Slot::Long(v))?;
            }
            Instruction::Lastore => {
                let val = frame.pop_long()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Long(val))?;
            }

            // ---- Float array ----
            Instruction::Faload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_float()?
                };
                frame.push(Slot::Float(v))?;
            }
            Instruction::Fastore => {
                let val = frame.pop_float()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Float(val))?;
            }

            // ---- Double array ----
            Instruction::Daload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_double()?
                };
                frame.push(Slot::Double(v))?;
            }
            Instruction::Dastore => {
                let val = frame.pop_double()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Double(val))?;
            }

            // ---- Reference array ----
            Instruction::Aaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize]
                };
                frame.push(v)?;
            }
            Instruction::Aastore => {
                let val = frame.pop()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, val)?;
            }

            // ---- Byte/boolean array (stored as Int, truncated to i8) ----
            Instruction::Baload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    i32::from(fields[idx_val as usize].as_int()? as i8)
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Bastore => {
                let val = i32::from(frame.pop_int()? as i8);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Char array (stored as Int, masked to u16) ----
            Instruction::Caload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    i32::from(fields[idx_val as usize].as_int()? as u16)
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Castore => {
                let val = i32::from(frame.pop_int()? as u16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Short array (stored as Int, truncated to i16) ----
            Instruction::Saload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    i32::from(fields[idx_val as usize].as_int()? as i16)
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Sastore => {
                let val = i32::from(frame.pop_int()? as i16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Switch ----
            Instruction::Tableswitch {
                default,
                low,
                high,
                offsets,
            } => {
                let key = frame.pop_int()?;
                let offset = if key >= *low && key <= *high {
                    offsets[(key - low) as usize]
                } else {
                    *default
                };
                jump!(offset);
            }
            Instruction::Lookupswitch { default, pairs } => {
                let key = frame.pop_int()?;
                let offset = pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map_or(*default, |(_, off)| *off);
                jump!(offset);
            }

            // ---- checkcast / instanceof ----
            Instruction::Checkcast(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(slot)?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = {
                            let ctx = registry.get(current_class)?;
                            resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                        };
                        let actual = heap.get(*r)?.class_name.clone();
                        if is_assignable_from(registry, loader, &actual, &target) {
                            frame.push(slot)?;
                        } else {
                            return Err(VmError::ClassCastException {
                                from: actual,
                                to: target,
                            });
                        }
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }
            Instruction::Instanceof(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(Slot::Int(0))?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = {
                            let ctx = registry.get(current_class)?;
                            resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                        };
                        let actual = heap.get(*r)?.class_name.clone();
                        let result =
                            i32::from(is_assignable_from(registry, loader, &actual, &target));
                        frame.push(Slot::Int(result))?;
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }

            // ---- athrow with exception table dispatch ----
            Instruction::Athrow => {
                let exception_ref = frame.pop_ref()?;
                let exc_class_name = heap.get(exception_ref)?.class_name.clone();
                propagate_java_exception!(exc_class_name, exception_ref, pc);
            }

            // ----------------------------------------------------------------
            // invokedynamic — resolve bootstrap method, dispatch based on
            // the bootstrap class (StringConcatFactory, LambdaMetafactory).
            // ----------------------------------------------------------------
            Instruction::Invokedynamic(cp_idx) => {
                let cp_idx_val = usize::from(cp_idx.0);

                // 1. Resolve InvokeDynamic CP entry.
                let (bsm_idx, call_name, call_desc) = {
                    let ctx = registry.get(current_class)?;
                    let cp = &ctx.constant_pool;
                    match cp.get(cp_idx_val).and_then(|e| e.as_ref()) {
                        Some(CpEntry::InvokeDynamic {
                            bootstrap_method_attr_index,
                            name_and_type_index,
                        }) => {
                            let (name, desc) =
                                resolve_name_and_type(cp, name_and_type_index.0 as usize)?;
                            (*bootstrap_method_attr_index as usize, name, desc)
                        }
                        _ => return Err(VmError::InvalidCpIndex { index: cp_idx_val }),
                    }
                };

                // 2. Look up the bootstrap method entry.
                let (bsm_class, bsm_args) = {
                    let ctx = registry.get(current_class)?;
                    let bsm_entry = ctx
                        .bootstrap_methods
                        .get(bsm_idx)
                        .ok_or(VmError::InvalidCpIndex { index: bsm_idx })?;
                    let (_kind, class, _name, _desc) =
                        resolve_method_handle(&ctx.constant_pool, bsm_entry.method_ref.0 as usize)?;
                    let args: Vec<duke_classfile::types::CpIndex> = bsm_entry.arguments.clone();
                    (class, args)
                };

                let _ = &call_name; // suppress unused warning for now

                // 3. Dispatch based on bootstrap method class.
                if bsm_class == "java/lang/invoke/StringConcatFactory" {
                    // --- StringConcatFactory.makeConcatWithConstants ---
                    let arg_count = parse_arg_count(&call_desc);
                    let arg_types = parse_arg_types(&call_desc);
                    // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                    let mut dynamic_args = vec![Slot::Int(0); arg_count];

                    for i in (0..arg_count).rev() {
                        dynamic_args[i] = frame.pop()?;
                    }

                    // Resolve recipe (first bootstrap arg) and constants (remaining).
                    let (recipe, constants) = {
                        let ctx = registry.get(current_class)?;
                        let cp = &ctx.constant_pool;
                        let recipe = if bsm_args.is_empty() {
                            String::new()
                        } else {
                            resolve_cp_string(cp, bsm_args[0].0 as usize)?
                        };
                        let mut consts = Vec::new();
                        for arg_idx in bsm_args.iter().skip(1) {
                            if let Ok(s) = resolve_cp_string(cp, arg_idx.0 as usize) {
                                consts.push(s);
                            }
                        }
                        (recipe, consts)
                    };

                    let result = execute_string_concat_recipe(
                        &recipe,
                        &dynamic_args,
                        &arg_types,
                        &constants,
                        heap,
                    )?;
                    frame.push(result)?;
                } else if bsm_class == "java/lang/invoke/LambdaMetafactory" {
                    // --- LambdaMetafactory.metafactory ---
                    // Bootstrap args: [MethodType erased, MethodHandle impl, MethodType specialized]

                    let (impl_kind, impl_class, impl_method, impl_desc) = {
                        let ctx = registry.get(current_class)?;
                        let cp = &ctx.constant_pool;
                        if bsm_args.len() < 3 {
                            return Err(VmError::Unimplemented {
                                mnemonic: "LambdaMetafactory requires 3 bootstrap args",
                            });
                        }
                        resolve_method_handle(cp, bsm_args[1].0 as usize)?
                    };

                    let sam_method = call_name.clone();

                    // Resolve erased SAM descriptor from bootstrap arg 0.
                    let sam_desc = {
                        let ctx = registry.get(current_class)?;
                        let cp = &ctx.constant_pool;
                        match cp.get(bsm_args[0].0 as usize).and_then(|e| e.as_ref()) {
                            Some(CpEntry::MethodType { descriptor_index }) => {
                                match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                                    Some(CpEntry::Utf8(s)) => s.clone(),
                                    _ => {
                                        return Err(VmError::InvalidCpIndex {
                                            index: descriptor_index.0 as usize,
                                        });
                                    }
                                }
                            }
                            _ => {
                                return Err(VmError::InvalidCpIndex {
                                    index: bsm_args[0].0 as usize,
                                });
                            }
                        }
                    };

                    // Pop captured variables from the stack.
                    let captured_count = parse_arg_count(&call_desc);
                    // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                    let mut captured_args = vec![Slot::Int(0); captured_count];

                    for i in (0..captured_count).rev() {
                        captured_args[i] = frame.pop()?;
                    }

                    let lambda_info = LambdaInfo {
                        impl_class: impl_class.clone(),
                        impl_method: impl_method.clone(),
                        impl_desc: impl_desc.clone(),
                        impl_kind,
                        sam_method,
                        sam_desc,
                        captured_count,
                    };
                    let lambda_class = registry.register_lambda(lambda_info);

                    let r = heap.allocate(lambda_class, captured_count);
                    for (i, slot) in captured_args.into_iter().enumerate() {
                        heap.get_mut(r)?.fields[i] = slot;
                    }

                    let _ = registry.ensure_loaded(&impl_class, loader);

                    frame.push(Slot::Reference(Some(r)))?;
                    if gc_allowed && heap.should_gc() {
                        let roots = gather_roots(frame, call_stack, registry);
                        heap.collect(&roots);
                        patch_forwarded_slots(frame, call_stack, registry, heap);
                    }
                } else {
                    // Unknown bootstrap method — pop args and push null.
                    let arg_count = parse_arg_count(&call_desc);
                    for _ in 0..arg_count {
                        frame.pop()?;
                    }
                    if !call_desc.ends_with(")V") {
                        frame.push(Slot::Reference(None))?;
                    }
                }
            }

            // ----------------------------------------------------------------
            // invokeinterface — like invokevirtual but resolves from
            // InterfaceMethodref and dispatches on the actual object class.
            // ----------------------------------------------------------------
            Instruction::Invokeinterface {
                index: cp_idx,
                count: _,
            } => {
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                if callee_name == "<clinit>" {
                    *idx += 1;
                    continue;
                }
                let arg_count = parse_arg_count(&callee_desc);
                // Peek at `this` (sits below the args) to determine the actual
                // runtime class without consuming the stack yet.  Each dispatch
                // path (native / lambda / bytecode) pops what it needs itself.
                let stack_len = frame.stack_len();
                let actual_class = if stack_len > arg_count {
                    let this_pos = stack_len - arg_count - 1;
                    if let Ok(Slot::Reference(Some(r))) = frame.peek_at(this_pos) {
                        heap.get(r).ok().map(|o| o.class_name.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
                .unwrap_or_else(|| callee_class.clone());

                // Try to find the method on the actual class (walking hierarchy).
                let resolved = resolve_method_in_hierarchy(
                    registry,
                    loader,
                    &actual_class,
                    &callee_name,
                    &callee_desc,
                );

                let (dispatch_class, callee_idx) = if let Some(pair) = resolved {
                    pair
                } else {
                    // Fall back to interface class hierarchy.
                    let iface_resolved = resolve_method_in_hierarchy(
                        registry,
                        loader,
                        &callee_class,
                        &callee_name,
                        &callee_desc,
                    );
                    match iface_resolved {
                        Some(pair) => pair,
                        None => {
                            // Check native registry — try actual class then interface class.
                            let native_kind = registry
                                .natives_mut()
                                .get_kind(&actual_class, &callee_name, &callee_desc)
                                .or_else(|| {
                                    registry.natives_mut().get_kind(
                                        &callee_class,
                                        &callee_name,
                                        &callee_desc,
                                    )
                                });
                            match native_kind {
                                Some(HandlerKind::Simple(handler)) => {
                                    // Native path: collect args + this into a Vec<Slot>
                                    // for the handler(&[Slot], ...) signature.
                                    // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                    let mut callee_args = vec![Slot::Int(0); arg_count];

                                    for i in (0..arg_count).rev() {
                                        callee_args[i] = frame.pop()?;
                                    }

                                    let this_slot = frame.pop()?;
                                    callee_args.insert(0, this_slot);
                                    #[cfg(feature = "telemetry")]
                                    let _native_start = std::time::Instant::now();
                                    let mut native_control = NativeControl::default();
                                    let result =
                                        handler(&callee_args, heap, stdout, &mut native_control);
                                    #[cfg(feature = "telemetry")]
                                    {
                                        registry.telemetry.native_boundary.record_call(
                                            &callee_class,
                                            &callee_name,
                                            _native_start.elapsed().as_nanos() as u64,
                                            result.is_err(),
                                        );
                                        // Native interface methods skip the bytecode
                                        // hierarchy walk — hierarchy_walk is always false here.
                                        registry.telemetry.dispatch_resolution.record(
                                            current_class,
                                            cp_idx.0,
                                            &actual_class,
                                            false,
                                        );
                                    }
                                    let result = match result {
                                        Ok(result) => result,
                                        Err(VmError::JavaException { class_name }) => {
                                            let exception_ref = materialize_java_exception_object(
                                                registry,
                                                loader,
                                                heap,
                                                &class_name,
                                            )?;
                                            propagate_java_exception!(
                                                class_name,
                                                exception_ref,
                                                pc
                                            );
                                        }
                                        Err(err) => return Err(err),
                                    };
                                    if let Some(outcome) =
                                        finish_native_call(&mut native_control, frame, idx, result)?
                                    {
                                        return Ok(outcome);
                                    }
                                    continue;
                                }
                                Some(HandlerKind::Callback(handler)) => {
                                    // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                    let mut callee_args = vec![Slot::Int(0); arg_count];

                                    for i in (0..arg_count).rev() {
                                        callee_args[i] = frame.pop()?;
                                    }
                                    let this_slot = frame.pop()?;
                                    callee_args.insert(0, this_slot);
                                    #[cfg(feature = "telemetry")]
                                    let _native_start = std::time::Instant::now();
                                    let mut native_control = NativeControl::default();
                                    let result = {
                                        let mut callback_ops =
                                            InterpreterCallbackOps { registry, loader };
                                        handler(
                                            &callee_args,
                                            heap,
                                            stdout,
                                            &mut native_control,
                                            &mut callback_ops,
                                        )
                                    };
                                    #[cfg(feature = "telemetry")]
                                    {
                                        registry.telemetry.native_boundary.record_call(
                                            &callee_class,
                                            &callee_name,
                                            _native_start.elapsed().as_nanos() as u64,
                                            result.is_err(),
                                        );
                                        registry.telemetry.dispatch_resolution.record(
                                            current_class,
                                            cp_idx.0,
                                            &actual_class,
                                            false,
                                        );
                                    }
                                    let result = match result {
                                        Ok(result) => result,
                                        Err(VmError::JavaException { class_name }) => {
                                            let exception_ref = materialize_java_exception_object(
                                                registry,
                                                loader,
                                                heap,
                                                &class_name,
                                            )?;
                                            propagate_java_exception!(
                                                class_name,
                                                exception_ref,
                                                pc
                                            );
                                        }
                                        Err(err) => return Err(err),
                                    };
                                    if let Some(outcome) =
                                        finish_native_call(&mut native_control, frame, idx, result)?
                                    {
                                        return Ok(outcome);
                                    }
                                    continue;
                                }
                                None => {} // fall through to lambda / no-op
                            }
                            // Check lambda registry for SAM dispatch.
                            if let Some(lambda_info) = registry.get_lambda(&actual_class).cloned()
                                && callee_name == lambda_info.sam_method
                            {
                                // Lambda path: collect args + this into a Vec<Slot>.
                                // PERF: Pre-allocate args and populate backwards to avoid intermediate .collect() and .reverse() allocs.

                                let mut callee_args = vec![Slot::Int(0); arg_count];

                                for i in (0..arg_count).rev() {
                                    callee_args[i] = frame.pop()?;
                                }
                                let this_slot = frame.pop()?;
                                callee_args.insert(0, this_slot);
                                let this_ref = match &callee_args[0] {
                                    Slot::Reference(Some(r)) => *r,
                                    _ => return Err(VmError::NullPointerException),
                                };
                                let obj = heap.get(this_ref)?;
                                let mut impl_args: Vec<Slot> = Vec::new();
                                for i in 0..lambda_info.captured_count {
                                    impl_args.push(obj.fields[i]);
                                }
                                impl_args.extend(callee_args[1..].iter().copied());

                                let _ = registry.ensure_loaded(&lambda_info.impl_class, loader);

                                if lambda_info.impl_kind == 6 {
                                    // invokeStatic dispatch
                                    let resolved = resolve_method_in_hierarchy(
                                        registry,
                                        loader,
                                        &lambda_info.impl_class,
                                        &lambda_info.impl_method,
                                        &lambda_info.impl_desc,
                                    );
                                    if let Some((dispatch_class, impl_idx)) = resolved {
                                        let (callee_pc_to_idx, callee_frame) = {
                                            let ctx = registry.get(&dispatch_class)?;
                                            let max_locals =
                                                usize::from(ctx.methods[impl_idx].max_locals);
                                            let max_stack =
                                                usize::from(ctx.methods[impl_idx].max_stack);
                                            let pci = std::sync::Arc::clone(
                                                &ctx.methods[impl_idx].pc_to_idx,
                                            );
                                            let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                            locals_buf.resize(max_locals, Slot::Int(0));
                                            for (i, slot) in impl_args.into_iter().enumerate() {
                                                locals_buf[i] = slot;
                                            }
                                            let f = Frame::from_pool_bufs(
                                                locals_buf, stack_buf, max_stack,
                                            );
                                            (pci, f)
                                        };
                                        activate_method_state(
                                            frame,
                                            method_idx,
                                            pc_to_idx,
                                            instructions,
                                            current_class,
                                            call_stack,
                                            registry,
                                            dispatch_class,
                                            impl_idx,
                                            callee_pc_to_idx,
                                            callee_frame,
                                            *idx + 1,
                                            #[cfg(feature = "telemetry")]
                                            current_method,
                                        )?;
                                        *idx = 0;
                                        continue;
                                    }
                                } else if lambda_info.impl_kind == 5 || lambda_info.impl_kind == 9 {
                                    // invokeVirtual / invokeInterface dispatch
                                    let resolved = resolve_method_in_hierarchy(
                                        registry,
                                        loader,
                                        &lambda_info.impl_class,
                                        &lambda_info.impl_method,
                                        &lambda_info.impl_desc,
                                    );
                                    if let Some((dispatch_class, impl_idx)) = resolved {
                                        let (callee_pc_to_idx, callee_frame) = {
                                            let ctx = registry.get(&dispatch_class)?;
                                            let max_locals =
                                                usize::from(ctx.methods[impl_idx].max_locals);
                                            let max_stack =
                                                usize::from(ctx.methods[impl_idx].max_stack);
                                            let pci = std::sync::Arc::clone(
                                                &ctx.methods[impl_idx].pc_to_idx,
                                            );
                                            let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                            locals_buf.resize(max_locals, Slot::Int(0));
                                            for (i, slot) in impl_args.into_iter().enumerate() {
                                                locals_buf[i] = slot;
                                            }
                                            let f = Frame::from_pool_bufs(
                                                locals_buf, stack_buf, max_stack,
                                            );
                                            (pci, f)
                                        };
                                        activate_method_state(
                                            frame,
                                            method_idx,
                                            pc_to_idx,
                                            instructions,
                                            current_class,
                                            call_stack,
                                            registry,
                                            dispatch_class,
                                            impl_idx,
                                            callee_pc_to_idx,
                                            callee_frame,
                                            *idx + 1,
                                            #[cfg(feature = "telemetry")]
                                            current_method,
                                        )?;
                                        *idx = 0;
                                        continue;
                                    }
                                    // Try native fallback for virtual/interface
                                    let lambda_native_kind = registry.natives_mut().get_kind(
                                        &lambda_info.impl_class,
                                        &lambda_info.impl_method,
                                        &lambda_info.impl_desc,
                                    );
                                    match lambda_native_kind {
                                        Some(HandlerKind::Simple(handler)) => {
                                            #[cfg(feature = "telemetry")]
                                            let _native_start = std::time::Instant::now();
                                            let mut native_control = NativeControl::default();
                                            let result = handler(
                                                &impl_args,
                                                heap,
                                                stdout,
                                                &mut native_control,
                                            );
                                            #[cfg(feature = "telemetry")]
                                            registry.telemetry.native_boundary.record_call(
                                                &lambda_info.impl_class,
                                                &lambda_info.impl_method,
                                                _native_start.elapsed().as_nanos() as u64,
                                                result.is_err(),
                                            );
                                            let result = match result {
                                                Ok(result) => result,
                                                Err(VmError::JavaException { class_name }) => {
                                                    let exception_ref =
                                                        materialize_java_exception_object(
                                                            registry,
                                                            loader,
                                                            heap,
                                                            &class_name,
                                                        )?;
                                                    propagate_java_exception!(
                                                        class_name,
                                                        exception_ref,
                                                        pc
                                                    );
                                                }
                                                Err(err) => return Err(err),
                                            };
                                            if let Some(outcome) = finish_native_call(
                                                &mut native_control,
                                                frame,
                                                idx,
                                                result,
                                            )? {
                                                return Ok(outcome);
                                            }
                                            continue;
                                        }
                                        Some(HandlerKind::Callback(handler)) => {
                                            #[cfg(feature = "telemetry")]
                                            let _native_start = std::time::Instant::now();
                                            let mut native_control = NativeControl::default();
                                            let result = {
                                                let mut callback_ops =
                                                    InterpreterCallbackOps { registry, loader };
                                                handler(
                                                    &impl_args,
                                                    heap,
                                                    stdout,
                                                    &mut native_control,
                                                    &mut callback_ops,
                                                )
                                            };
                                            #[cfg(feature = "telemetry")]
                                            registry.telemetry.native_boundary.record_call(
                                                &lambda_info.impl_class,
                                                &lambda_info.impl_method,
                                                _native_start.elapsed().as_nanos() as u64,
                                                result.is_err(),
                                            );
                                            let result = match result {
                                                Ok(result) => result,
                                                Err(VmError::JavaException { class_name }) => {
                                                    let exception_ref =
                                                        materialize_java_exception_object(
                                                            registry,
                                                            loader,
                                                            heap,
                                                            &class_name,
                                                        )?;
                                                    propagate_java_exception!(
                                                        class_name,
                                                        exception_ref,
                                                        pc
                                                    );
                                                }
                                                Err(err) => return Err(err),
                                            };
                                            if let Some(outcome) = finish_native_call(
                                                &mut native_control,
                                                frame,
                                                idx,
                                                result,
                                            )? {
                                                return Ok(outcome);
                                            }
                                            continue;
                                        }
                                        None => {}
                                    }
                                }
                                *idx += 1;
                                continue;
                            }
                            // No-op fallback.
                            *idx += 1;
                            continue;
                        }
                    }
                };

                // Bytecode execution path — Pattern B: pop directly into locals_buf.
                #[cfg(feature = "telemetry")]
                registry.telemetry.dispatch_resolution.record(
                    current_class,
                    cp_idx.0,
                    &dispatch_class,
                    dispatch_class != actual_class,
                );
                let (callee_pc_to_idx, callee_frame) = {
                    let ctx = registry.get(&dispatch_class)?;
                    let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                    let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                    let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                    let (mut locals_buf, stack_buf) = frame_pool.acquire();
                    locals_buf.resize(max_locals, Slot::Int(0));
                    if arg_count + 1 > max_locals {
                        return Err(VmError::LocalOutOfBounds {
                            index: arg_count + 1,
                            max_locals,
                        });
                    }
                    // Pop method args in reverse (stack top = last arg) into locals[1..=arg_count].
                    for i in (1..=arg_count).rev() {
                        locals_buf[i] = frame.pop()?;
                    }
                    locals_buf[0] = frame.pop()?; // `this`
                    let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                    (pci, f)
                };
                activate_method_state(
                    frame,
                    method_idx,
                    pc_to_idx,
                    instructions,
                    current_class,
                    call_stack,
                    registry,
                    dispatch_class,
                    callee_idx,
                    callee_pc_to_idx,
                    callee_frame,
                    *idx + 1,
                    #[cfg(feature = "telemetry")]
                    current_method,
                )?;
                *idx = 0;
                continue;
            }

            // ----------------------------------------------------------------
            // multianewarray — allocate multi-dimensional arrays.
            // ----------------------------------------------------------------
            Instruction::Multianewarray {
                index: cp_idx,
                dimensions,
            } => {
                let element_type = {
                    let ctx = registry.get(current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };

                // Pop dimension sizes from stack (first popped is rightmost dimension).
                let mut dims: Vec<i32> = Vec::new();
                for _ in 0..*dimensions {
                    dims.push(frame.pop_int()?);
                }
                dims.reverse(); // Now dims[0] is outermost.

                // Check for negative sizes.
                for &d in &dims {
                    if d < 0 {
                        return Err(VmError::NegativeArraySize { size: d });
                    }
                }

                // Recursive allocation helper.
                fn alloc_multi(
                    heap: &mut duke_gc::Heap,
                    dims: &[i32],
                    depth: usize,
                    type_name: &str,
                ) -> u64 {
                    let size = dims[depth] as usize;
                    let r = heap.allocate(type_name.to_string(), size);
                    if depth < dims.len() - 1 {
                        // Not the innermost — fill with references to sub-arrays.
                        let inner_type = &type_name[1..]; // Strip one '[' for inner dimension.
                        for i in 0..size {
                            let inner = alloc_multi(heap, dims, depth + 1, inner_type);
                            heap.get_mut(r).unwrap().fields[i] = Slot::Reference(Some(inner));
                        }
                    }
                    r
                }

                let r = alloc_multi(heap, &dims, 0, &element_type);
                frame.push(Slot::Reference(Some(r)))?;
                if gc_allowed && heap.should_gc() {
                    let roots = gather_roots(frame, call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(frame, call_stack, registry, heap);
                }
            }

            // ---- monitor (no-op, single-threaded) ----
            Instruction::Monitorenter | Instruction::Monitorexit => {
                let _obj = frame.pop()?;
            }

            other => {
                return Err(VmError::Unimplemented {
                    mnemonic: other.mnemonic(),
                });
            }
        }

        #[cfg(feature = "telemetry")]
        {
            let elapsed = _telem_start.elapsed().as_nanos() as u64;
            registry.telemetry.bytecode_cost.record(
                _telem_name,
                current_class,
                current_method,
                _telem_pc,
                elapsed,
            );
        }

        *idx += 1;
    }
}

struct CompletionVm {
    registry: ClassRegistry,
    heap: duke_gc::Heap,
    output: Vec<u8>,
    live_workers: usize,
}

#[derive(Default)]
struct CompletionRuntime {
    threads: ThreadRuntime,
    handles: HashMap<i32, std::thread::JoinHandle<VmResult<()>>>,
}

fn resolve_thread_entry(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &duke_gc::Heap,
    thread_ref: u64,
) -> VmResult<Option<(String, usize, Vec<Slot>)>> {
    let actual_class = heap.get(thread_ref)?.class_name.clone();
    if actual_class != "java/lang/Thread"
        && let Some((dispatch_class, method_idx)) =
            resolve_method_in_hierarchy(registry, loader, &actual_class, "run", "()V")
    {
        return Ok(Some((
            dispatch_class,
            method_idx,
            vec![Slot::Reference(Some(thread_ref))],
        )));
    }

    let target_ref = match heap.get(thread_ref)?.fields.get(THREAD_TARGET_SLOT) {
        Some(Slot::Reference(Some(target_ref))) => *target_ref,
        _ => return Ok(None),
    };
    let target_class = heap.get(target_ref)?.class_name.clone();
    let (dispatch_class, method_idx) =
        resolve_method_in_hierarchy(registry, loader, &target_class, "run", "()V").ok_or_else(
            || VmError::AbstractMethodError {
                class_name: target_class.clone(),
                method_name: "run".to_string(),
            },
        )?;
    Ok(Some((
        dispatch_class,
        method_idx,
        vec![Slot::Reference(Some(target_ref))],
    )))
}

fn join_java_thread(
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    thread_id: i32,
) -> VmResult<()> {
    loop {
        let handle = {
            let mut runtime = runtime.lock().unwrap();
            let is_finished = runtime
                .threads
                .records()
                .iter()
                .find(|record| record.thread_id == thread_id)
                .is_none_or(|record| record.finished);
            if is_finished {
                return Ok(());
            }
            runtime.handles.remove(&thread_id)
        };

        if let Some(handle) = handle {
            return match handle.join() {
                Ok(result) => result,
                Err(payload) => std::panic::resume_unwind(payload),
            };
        }

        std::thread::yield_now();
    }
}

fn wait_for_all_java_threads(
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
) -> VmResult<()> {
    loop {
        let handles = {
            let mut runtime = runtime.lock().unwrap();
            if runtime.handles.is_empty() {
                return Ok(());
            }
            runtime
                .handles
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };

        for handle in handles {
            match handle.join() {
                Ok(result) => result?,
                Err(payload) => std::panic::resume_unwind(payload),
            }
        }
    }
}

fn handle_thread_action(
    action: NativeThreadAction,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> VmResult<()> {
    match action {
        NativeThreadAction::Start { thread_ref } => {
            spawn_java_thread(shared, runtime, loader, thread_ref)
        }
        NativeThreadAction::Sleep(duration) => {
            std::thread::sleep(duration);
            Ok(())
        }
        NativeThreadAction::Join { thread_id } => join_java_thread(runtime, thread_id),
    }
}

fn run_thread_to_completion(
    mut state: ExecutionState,
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
) -> VmResult<()> {
    loop {
        let mut shared_guard = shared.lock().unwrap();
        let CompletionVm {
            registry,
            heap,
            output,
            live_workers,
        } = &mut *shared_guard;
        let outcome = run_execution(
            &mut state,
            registry,
            loader.as_ref(),
            heap,
            output,
            *live_workers == 0,
        )?;
        drop(shared_guard);

        match outcome {
            ExecutionOutcome::Returned(_) => return Ok(()),
            ExecutionOutcome::ThreadAction(action) => {
                handle_thread_action(action, shared, runtime, loader)?;
            }
        }
    }
}

fn spawn_java_thread(
    shared: &std::sync::Arc<std::sync::Mutex<CompletionVm>>,
    runtime: &std::sync::Arc<std::sync::Mutex<CompletionRuntime>>,
    loader: &std::sync::Arc<dyn ClassLoader + Send + Sync>,
    thread_ref: u64,
) -> VmResult<()> {
    {
        let mut shared_guard = shared.lock().unwrap();
        let thread = shared_guard.heap.get_mut(thread_ref)?;
        let already_started = matches!(
            thread.fields.get(THREAD_ID_SLOT),
            Some(Slot::Int(thread_id)) if *thread_id >= 0
        );
        drop(shared_guard);
        if already_started {
            return Ok(());
        }
    }

    let thread_id = {
        let mut runtime = runtime.lock().unwrap();
        let thread_id = runtime.threads.allocate_thread_id();
        runtime
            .threads
            .register(ThreadRecord::new(thread_ref, thread_id));
        thread_id
    };

    let entry = {
        let mut shared_guard = shared.lock().unwrap();
        {
            let thread = shared_guard.heap.get_mut(thread_ref)?;
            thread.fields[THREAD_ID_SLOT] = Slot::Int(thread_id);
        }
        let CompletionVm {
            registry,
            heap,
            live_workers,
            ..
        } = &mut *shared_guard;
        let entry = resolve_thread_entry(registry, loader.as_ref(), heap, thread_ref)?;
        if entry.is_some() {
            *live_workers += 1;
        }
        drop(shared_guard);
        entry
    };
    let Some((dispatch_class, method_idx, args)) = entry else {
        let _ = runtime.lock().unwrap().threads.mark_finished(thread_id);
        return Ok(());
    };

    let state = {
        let shared = shared.lock().unwrap();
        ExecutionState::new(&shared.registry, &dispatch_class, "run", method_idx, &args)?
    };

    let shared_clone = std::sync::Arc::clone(shared);
    let runtime_clone = std::sync::Arc::clone(runtime);
    let loader_clone = std::sync::Arc::clone(loader);
    let handle = std::thread::spawn(move || {
        let result = run_thread_to_completion(state, &shared_clone, &runtime_clone, &loader_clone);
        {
            let mut shared = shared_clone.lock().unwrap();
            shared.live_workers = shared.live_workers.saturating_sub(1);
        }
        let _ = runtime_clone
            .lock()
            .unwrap()
            .threads
            .mark_finished_by_java_ref(thread_ref);
        result
    });
    runtime.lock().unwrap().handles.insert(thread_id, handle);
    Ok(())
}

/// Execute a Java entrypoint and keep the VM alive until any spawned worker
/// threads have either finished or been joined.
///
/// # Errors
///
/// Returns `VmError` if class resolution, method dispatch, or bytecode
/// execution fails in any thread.
///
/// # Panics
///
/// Panics if a `Mutex` protecting shared VM state is poisoned by a
/// panicking thread, or if the `Arc` cannot be unwound after all threads
/// have joined.
#[allow(clippy::too_many_arguments)]
pub fn execute_class_to_completion<L>(
    registry: &mut ClassRegistry,
    loader: L,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>>
where
    L: ClassLoader + Send + Sync + 'static,
{
    let loader: std::sync::Arc<dyn ClassLoader + Send + Sync> = std::sync::Arc::new(loader);
    if registry
        .natives()
        .get_kind(class_name, method_name, descriptor)
        .is_some()
    {
        return execute_class(
            registry,
            loader.as_ref(),
            heap,
            stdout,
            class_name,
            method_name,
            descriptor,
            args,
        );
    }

    let mut vm = CompletionVm {
        registry: std::mem::take(registry),
        heap: std::mem::replace(heap, duke_gc::Heap::new()),
        output: Vec::new(),
        live_workers: 0,
    };

    let mut state = match prepare_execution_state(
        &mut vm.registry,
        loader.as_ref(),
        &mut vm.heap,
        &mut vm.output,
        class_name,
        method_name,
        descriptor,
        args,
    ) {
        Ok(state) => state,
        Err(err) => {
            *registry = vm.registry;
            *heap = vm.heap;
            return Err(err);
        }
    };

    let shared = std::sync::Arc::new(std::sync::Mutex::new(vm));
    let runtime = std::sync::Arc::new(std::sync::Mutex::new(CompletionRuntime::default()));

    let run_result: VmResult<Option<Slot>> = loop {
        let mut shared_guard = shared.lock().unwrap();
        let CompletionVm {
            registry,
            heap,
            output,
            live_workers,
        } = &mut *shared_guard;
        let outcome = run_execution(
            &mut state,
            registry,
            loader.as_ref(),
            heap,
            output,
            *live_workers == 0,
        )?;
        drop(shared_guard);

        match outcome {
            ExecutionOutcome::Returned(result) => break Ok(result),
            ExecutionOutcome::ThreadAction(action) => {
                handle_thread_action(action, &shared, &runtime, &loader)?;
            }
        }
    };

    let wait_result = wait_for_all_java_threads(&runtime);
    let Ok(shared) = std::sync::Arc::try_unwrap(shared) else {
        panic!("completion runtime released shared VM state")
    };
    let Ok(shared) = shared.into_inner() else {
        panic!("shared VM mutex poisoned")
    };
    let flush_result = stdout
        .write_all(&shared.output)
        .map_err(|_| VmError::Unimplemented {
            mnemonic: "output flush",
        });
    *registry = shared.registry;
    *heap = shared.heap;

    match run_result {
        Ok(result) => {
            wait_result?;
            flush_result?;
            Ok(result)
        }
        Err(err) => {
            let _ = wait_result;
            let _ = flush_result;
            Err(err)
        }
    }
}

/// Saved state of a caller frame suspended during a method call.
struct CallFrame {
    frame: Frame,
    method_idx: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    resume_idx: usize,
    /// Class that was executing when this frame was pushed.
    class_name: String,
}

/// Build a [`ClassContext`] from a parsed [`duke_classfile::ClassFile`].
///
/// Decodes all methods with a Code attribute and extracts field metadata.
/// Methods without Code (abstract, native) are silently skipped.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn build_class_context(cf: &duke_classfile::ClassFile) -> ClassContext {
    use duke_bytecode::decode;
    use duke_classfile::access_flags::FieldAccessFlags;
    use duke_classfile::types::{AttributeData, CpEntry};

    // Resolve this_class -> class name string.
    let class_name = {
        let entry = cf
            .constant_pool
            .get(cf.this_class.0 as usize)
            .and_then(|e| e.as_ref());
        if let Some(CpEntry::Class { name_index }) = entry {
            match cf
                .constant_pool
                .get(name_index.0 as usize)
                .and_then(|e| e.as_ref())
            {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => String::new(),
            }
        } else {
            String::new()
        }
    };

    let methods = cf
        .methods
        .iter()
        .filter_map(|m| {
            let name = match cf.constant_pool.get(m.name_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let descriptor = match cf.constant_pool.get(m.descriptor_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let code = m.attributes.iter().find_map(|a| {
                if let AttributeData::Code(c) = &a.data {
                    Some(c)
                } else {
                    None
                }
            })?;
            let instructions = decode(&code.code).ok()?;
            let exception_table: Vec<ExceptionEntry> = code
                .exception_table
                .iter()
                .map(|e| {
                    let catch_type = if e.catch_type.0 == 0 {
                        None // catch-all (finally)
                    } else {
                        match cf
                            .constant_pool
                            .get(e.catch_type.0 as usize)
                            .and_then(|x| x.as_ref())
                        {
                            Some(CpEntry::Class { name_index }) => {
                                match cf
                                    .constant_pool
                                    .get(name_index.0 as usize)
                                    .and_then(|x| x.as_ref())
                                {
                                    Some(CpEntry::Utf8(s)) => Some(s.clone()),
                                    _ => None,
                                }
                            }
                            _ => None,
                        }
                    };
                    ExceptionEntry {
                        start_pc: e.start_pc,
                        end_pc: e.end_pc,
                        handler_pc: e.handler_pc,
                        catch_type,
                    }
                })
                .collect();
            let pc_to_idx_map: std::collections::HashMap<usize, usize> = instructions
                .iter()
                .enumerate()
                .map(|(i, &(pc, _))| (pc, i))
                .collect();
            Some(MethodEntry {
                name,
                descriptor,
                instructions: instructions.into(),
                max_stack: code.max_stack,
                max_locals: code.max_locals,
                exception_table,
                pc_to_idx: std::sync::Arc::new(pc_to_idx_map),
            })
        })
        .collect();

    let mut fields = Vec::new();
    let mut static_count = 0usize;
    let mut instance_count = 0usize;

    for f in &cf.fields {
        let name = match cf.constant_pool.get(f.name_index.0 as usize) {
            Some(Some(CpEntry::Utf8(s))) => s.clone(),
            _ => continue,
        };
        let descriptor = match cf.constant_pool.get(f.descriptor_index.0 as usize) {
            Some(Some(CpEntry::Utf8(s))) => s.clone(),
            _ => continue,
        };
        let is_static = f.access_flags.contains(FieldAccessFlags::STATIC);
        if is_static {
            static_count += 1;
        } else {
            instance_count += 1;
        }
        fields.push(FieldEntry {
            name,
            descriptor,
            is_static,
        });
    }

    // Resolve super_class: if the index is 0, this is java/lang/Object (no super).
    let super_class = if cf.super_class.0 != 0 {
        resolve_class_name(&cf.constant_pool, cf.super_class.0 as usize).ok()
    } else {
        None
    };

    // Resolve directly-implemented interfaces.
    let interfaces: Vec<String> = cf
        .interfaces
        .iter()
        .filter_map(|idx| resolve_class_name(&cf.constant_pool, idx.0 as usize).ok())
        .collect();

    // Extract BootstrapMethods from class-level attributes.
    let bootstrap_methods = cf
        .attributes
        .iter()
        .find_map(|a| {
            if let AttributeData::BootstrapMethods(entries) = &a.data {
                Some(entries.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    ClassContext {
        class_name,
        super_class,
        interfaces,
        constant_pool: cf.constant_pool.clone(),
        methods,
        fields,
        static_fields: vec![Slot::Int(0); static_count],
        instance_field_count: instance_count,
        bootstrap_methods,
    }
}

/// Resolve a CP Class entry to its name string.
fn resolve_class_name(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Class { name_index }) => {
            match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s.clone()),
                _ => Err(VmError::InvalidCpIndex {
                    index: name_index.0 as usize,
                }),
            }
        }
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

pub(crate) fn internal_name_to_binary_name(name: &str) -> String {
    name.replace('/', ".")
}

pub(crate) fn binary_name_to_internal_name(name: &str) -> String {
    name.replace('.', "/")
}

fn cp_utf8_string(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Utf8(s)) => Ok(s.clone()),
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

fn inspect_reflected_class(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    internal_name: &str,
) -> VmResult<ReflectedClassInfo> {
    use duke_classfile::access_flags::{FieldAccessFlags, MethodAccessFlags};

    if let Ok(bytes) = loader.find_class(internal_name)
        && let Ok(class_file) = duke_classfile::parse(&bytes)
    {
        let methods = class_file
            .methods
            .iter()
            .filter_map(|method| {
                let name =
                    cp_utf8_string(&class_file.constant_pool, method.name_index.0 as usize).ok()?;
                let descriptor = cp_utf8_string(
                    &class_file.constant_pool,
                    method.descriptor_index.0 as usize,
                )
                .ok()?;
                Some(ReflectedMethodInfo {
                    name,
                    descriptor,
                    is_public: method.access_flags.contains(MethodAccessFlags::PUBLIC),
                    is_static: method.access_flags.contains(MethodAccessFlags::STATIC),
                })
            })
            .collect();
        let fields = class_file
            .fields
            .iter()
            .filter_map(|field| {
                let name =
                    cp_utf8_string(&class_file.constant_pool, field.name_index.0 as usize).ok()?;
                let descriptor =
                    cp_utf8_string(&class_file.constant_pool, field.descriptor_index.0 as usize)
                        .ok()?;
                Some(ReflectedFieldInfo {
                    name,
                    descriptor,
                    is_public: field.access_flags.contains(FieldAccessFlags::PUBLIC),
                    is_static: field.access_flags.contains(FieldAccessFlags::STATIC),
                })
            })
            .collect();
        return Ok(ReflectedClassInfo {
            internal_name: internal_name.to_string(),
            binary_name: internal_name_to_binary_name(internal_name),
            methods,
            fields,
        });
    }

    registry.ensure_loaded(internal_name, loader)?;
    let ctx = registry.get(internal_name)?;
    let methods = ctx
        .methods
        .iter()
        .map(|method| ReflectedMethodInfo {
            name: method.name.clone(),
            descriptor: method.descriptor.clone(),
            is_public: true,
            is_static: false,
        })
        .collect();
    let fields = ctx
        .fields
        .iter()
        .map(|field| ReflectedFieldInfo {
            name: field.name.clone(),
            descriptor: field.descriptor.clone(),
            is_public: true,
            is_static: field.is_static,
        })
        .collect();

    Ok(ReflectedClassInfo {
        internal_name: internal_name.to_string(),
        binary_name: internal_name_to_binary_name(internal_name),
        methods,
        fields,
    })
}

const REFLECTION_MEMBER_DECLARING_CLASS_FIELD: usize = 0;
const REFLECTION_MEMBER_NAME_FIELD: usize = 1;
const REFLECTION_MEMBER_DESCRIPTOR_FIELD: usize = 2;
const REFLECTION_MEMBER_PUBLIC_FIELD: usize = 3;
const REFLECTION_MEMBER_STATIC_FIELD: usize = 4;

pub(crate) fn allocate_class_object(
    heap: &mut duke_gc::Heap,
    internal_name: &str,
) -> VmResult<u64> {
    let class_ref = heap.allocate("java/lang/Class".to_string(), 0);
    heap.get_mut(class_ref)?.string_value = Some(internal_name.to_string());
    Ok(class_ref)
}

pub(crate) fn class_internal_name_from_ref(
    heap: &duke_gc::Heap,
    class_ref: u64,
) -> VmResult<String> {
    heap.get(class_ref)?
        .string_value
        .clone()
        .ok_or(VmError::InvalidRef { address: class_ref })
}

pub(crate) fn allocate_reference_array(
    heap: &mut duke_gc::Heap,
    array_class_name: &str,
    elements: &[u64],
) -> VmResult<u64> {
    let array_ref = heap.allocate(array_class_name.to_string(), elements.len());
    {
        let array_obj = heap.get_mut(array_ref)?;
        for slot in &mut array_obj.fields {
            *slot = Slot::Reference(None);
        }
    }
    for (idx, element_ref) in elements.iter().enumerate() {
        heap.write_field(array_ref, idx, Slot::Reference(Some(*element_ref)))?;
    }
    Ok(array_ref)
}

pub(crate) fn allocate_reflection_member_object(
    heap: &mut duke_gc::Heap,
    member_class_name: &str,
    declaring_internal_name: &str,
    name: &str,
    descriptor: &str,
    is_public: bool,
    is_static: bool,
) -> VmResult<u64> {
    let member_ref = heap.allocate(member_class_name.to_string(), 5);
    let declaring_class_ref = allocate_class_object(heap, declaring_internal_name)?;
    let name_ref = heap.allocate_string(name.to_string());
    let descriptor_ref = heap.allocate_string(descriptor.to_string());
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_DECLARING_CLASS_FIELD,
        Slot::Reference(Some(declaring_class_ref)),
    )?;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_NAME_FIELD,
        Slot::Reference(Some(name_ref)),
    )?;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_DESCRIPTOR_FIELD,
        Slot::Reference(Some(descriptor_ref)),
    )?;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_PUBLIC_FIELD,
        Slot::Int(i32::from(is_public)),
    )?;
    heap.write_field(
        member_ref,
        REFLECTION_MEMBER_STATIC_FIELD,
        Slot::Int(i32::from(is_static)),
    )?;
    Ok(member_ref)
}

pub(crate) fn reflection_member_name_slot(heap: &duke_gc::Heap, member_ref: u64) -> VmResult<Slot> {
    heap.get(member_ref)?
        .fields
        .get(REFLECTION_MEMBER_NAME_FIELD)
        .copied()
        .ok_or(VmError::InvalidRef {
            address: member_ref,
        })
}

pub(crate) fn reflection_array_elements(
    heap: &duke_gc::Heap,
    args_slot: Slot,
) -> VmResult<Vec<Slot>> {
    match args_slot {
        Slot::Reference(None) => Ok(Vec::new()),
        Slot::Reference(Some(array_ref)) => Ok(heap.get(array_ref)?.fields.clone()),
        _ => Err(VmError::TypeMismatch {
            expected: "reference array",
            got: "other",
        }),
    }
}

pub(crate) struct ReflectedMethodHandle {
    pub(crate) declaring_internal_name: String,
    pub(crate) method_name: String,
    pub(crate) descriptor: String,
    pub(crate) is_public: bool,
    pub(crate) is_static: bool,
}

pub(crate) fn reflected_method_handle(
    heap: &duke_gc::Heap,
    method_ref: u64,
) -> VmResult<ReflectedMethodHandle> {
    let method_obj = heap.get(method_ref)?;
    let Some(Slot::Reference(Some(declaring_class_ref))) = method_obj
        .fields
        .get(REFLECTION_MEMBER_DECLARING_CLASS_FIELD)
        .copied()
    else {
        return Err(VmError::InvalidRef {
            address: method_ref,
        });
    };
    let Some(Slot::Reference(Some(name_ref))) =
        method_obj.fields.get(REFLECTION_MEMBER_NAME_FIELD).copied()
    else {
        return Err(VmError::InvalidRef {
            address: method_ref,
        });
    };
    let Some(Slot::Reference(Some(descriptor_ref))) = method_obj
        .fields
        .get(REFLECTION_MEMBER_DESCRIPTOR_FIELD)
        .copied()
    else {
        return Err(VmError::InvalidRef {
            address: method_ref,
        });
    };
    let is_public = matches!(
        method_obj.fields.get(REFLECTION_MEMBER_PUBLIC_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );
    let is_static = matches!(
        method_obj.fields.get(REFLECTION_MEMBER_STATIC_FIELD),
        Some(Slot::Int(value)) if *value != 0
    );

    Ok(ReflectedMethodHandle {
        declaring_internal_name: class_internal_name_from_ref(heap, declaring_class_ref)?,
        method_name: heap
            .get(name_ref)?
            .string_value
            .clone()
            .ok_or(VmError::NullPointerException)?,
        descriptor: heap
            .get(descriptor_ref)?
            .string_value
            .clone()
            .ok_or(VmError::NullPointerException)?,
        is_public,
        is_static,
    })
}

pub(crate) fn build_reflection_invoke_args(
    heap: &duke_gc::Heap,
    target_slot: Slot,
    descriptor: &str,
    invoke_arg_slots: Vec<Slot>,
    is_static: bool,
) -> VmResult<Vec<Slot>> {
    let arg_types = parse_arg_types(descriptor);
    if arg_types.len() != invoke_arg_slots.len() {
        return Err(VmError::TypeMismatch {
            expected: "matching reflective argument count",
            got: "different count",
        });
    }

    let mut invoke_args = Vec::with_capacity(arg_types.len() + usize::from(!is_static));
    if !is_static {
        match target_slot {
            Slot::Reference(Some(_)) => invoke_args.push(target_slot),
            _ => return Err(VmError::NullPointerException),
        }
    }
    for (descriptor, arg) in arg_types.iter().copied().zip(invoke_arg_slots) {
        invoke_args.push(unbox_reflection_argument(heap, descriptor, arg)?);
        if matches!(descriptor, 'J' | 'D') {
            invoke_args.push(Slot::Int(0));
        }
    }
    Ok(invoke_args)
}

fn unbox_reflection_argument(heap: &duke_gc::Heap, descriptor: char, arg: Slot) -> VmResult<Slot> {
    match descriptor {
        'L' | '[' => match arg {
            Slot::Reference(_) => Ok(arg),
            _ => Err(VmError::TypeMismatch {
                expected: "reference",
                got: "other",
            }),
        },
        'B' | 'C' | 'I' | 'S' | 'Z' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(VmError::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Int(value)) => Ok(Slot::Int(*value)),
                _ => Err(VmError::TypeMismatch {
                    expected: "boxed int-like primitive",
                    got: "other",
                }),
            }
        }
        'J' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(VmError::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Long(value)) => Ok(Slot::Long(*value)),
                _ => Err(VmError::TypeMismatch {
                    expected: "boxed long",
                    got: "other",
                }),
            }
        }
        'F' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(VmError::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Float(value)) => Ok(Slot::Float(*value)),
                _ => Err(VmError::TypeMismatch {
                    expected: "boxed float",
                    got: "other",
                }),
            }
        }
        'D' => {
            let Slot::Reference(Some(obj_ref)) = arg else {
                return Err(VmError::NullPointerException);
            };
            match heap.get(obj_ref)?.fields.first() {
                Some(Slot::Double(value)) => Ok(Slot::Double(*value)),
                _ => Err(VmError::TypeMismatch {
                    expected: "boxed double",
                    got: "other",
                }),
            }
        }
        _ => Err(VmError::Unimplemented {
            mnemonic: "reflection primitive unboxing",
        }),
    }
}

pub(crate) fn descriptor_return_type(descriptor: &str) -> char {
    descriptor
        .split_once(')')
        .and_then(|(_, ret)| ret.chars().next())
        .unwrap_or('V')
}

pub(crate) fn box_reflection_return_value(
    heap: &mut duke_gc::Heap,
    return_type: char,
    result: Option<Slot>,
) -> VmResult<Slot> {
    match return_type {
        'V' => Ok(Slot::Reference(None)),
        'L' | '[' => Ok(result.unwrap_or(Slot::Reference(None))),
        'B' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "byte result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Byte".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'C' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "char result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Character".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'D' => {
            let Some(Slot::Double(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "double result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Double".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Double(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'F' => {
            let Some(Slot::Float(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "float result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Float".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Float(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'I' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "int result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'J' => {
            let Some(Slot::Long(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "long result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Long".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Long(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'S' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "short result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Short".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        'Z' => {
            let Some(Slot::Int(value)) = result else {
                return Err(VmError::TypeMismatch {
                    expected: "boolean result",
                    got: "other",
                });
            };
            let boxed_ref = heap.allocate("java/lang/Boolean".to_string(), 1);
            heap.get_mut(boxed_ref)?.fields[0] = Slot::Int(value);
            Ok(Slot::Reference(Some(boxed_ref)))
        }
        _ => Err(VmError::Unimplemented {
            mnemonic: "reflection primitive boxing",
        }),
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

/// Check if `from` is a subtype of `to` (i.e., `from` can be assigned where `to` is expected).
///
/// Walks the class hierarchy from `from` upward through superclasses.
/// Returns `true` if `to` is found in the chain, or if `to` is `"java/lang/Object"`.
/// Return `true` if a reference of type `from` can be used where `to` is expected.
///
/// BFS over the full type graph (superclass + all implemented interfaces at each
/// level), so `String instanceof Comparable` resolves correctly.
pub(crate) fn is_assignable_from(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    from: &str,
    to: &str,
) -> bool {
    if from == to || to == "java/lang/Object" {
        return true;
    }
    // Arrays implement Cloneable and Serializable; everything else is Object.
    if from.starts_with('[') {
        return matches!(to, "java/lang/Cloneable" | "java/io/Serializable");
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    queue.push_back(from.to_string());

    while let Some(current) = queue.pop_front() {
        if !visited.insert(current.clone()) {
            continue;
        }
        let _ = registry.ensure_loaded(&current, loader);
        let (super_class, interfaces) = match registry.get(&current) {
            Ok(ctx) => (ctx.super_class.clone(), ctx.interfaces.clone()),
            Err(_) => continue,
        };
        if let Some(sc) = super_class {
            if sc == to {
                return true;
            }
            queue.push_back(sc);
        }
        for iface in interfaces {
            if iface == to {
                return true;
            }
            queue.push_back(iface);
        }
    }
    false
}

/// Search a method's exception table for a handler matching the given pc and exception class.
///
/// Uses hierarchy-aware type checking: a `catch(Exception)` will match a thrown
/// `RuntimeException` because `RuntimeException` is a subclass of `Exception`.
pub(crate) fn find_exception_handler(
    exception_table: &[ExceptionEntry],
    pc: usize,
    class_name: &str,
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
) -> Option<u16> {
    exception_table.iter().find_map(|entry| {
        let in_range = pc >= entry.start_pc as usize && pc < entry.end_pc as usize;
        #[allow(clippy::option_if_let_else)] // match is clearer with &mut registry
        let type_matches = match &entry.catch_type {
            None => true, // catch-all (finally)
            Some(ct) => is_assignable_from(registry, loader, class_name, ct),
        };
        if in_range && type_matches {
            Some(entry.handler_pc)
        } else {
            None
        }
    })
}

/// Walk the class hierarchy to find a method by name and descriptor.
/// Returns `(class_name_where_found, method_index)` or `None`.
fn resolve_method_in_hierarchy(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    start_class: &str,
    method_name: &str,
    method_desc: &str,
) -> Option<(String, usize)> {
    let mut current = start_class.to_string();
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(current.clone()) {
            return None; // circular — bail
        }
        let _ = registry.ensure_loaded(&current, loader);
        match registry.get(&current) {
            Ok(ctx) => {
                if let Some(idx) = ctx
                    .methods
                    .iter()
                    .position(|m| m.name == method_name && m.descriptor == method_desc)
                {
                    return Some((current, idx));
                }
                match &ctx.super_class {
                    Some(s) => current.clone_from(s),
                    None => return None,
                }
            }
            Err(_) => return None,
        }
    }
}

/// Resolve a constant pool Methodref to (`class_name`, `method_name`, `descriptor`).
pub(crate) fn resolve_methodref(
    cp: &[Option<CpEntry>],
    idx: usize,
) -> VmResult<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(
            CpEntry::Methodref {
                class_index,
                name_and_type_index,
            }
            | CpEntry::InterfaceMethodref {
                class_index,
                name_and_type_index,
            },
        ) => {
            let class_name = match cp.get(class_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Class { name_index }) => {
                    match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    }
                }
                _ => return Err(VmError::InvalidMethodref { index: idx }),
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    };
                    Ok((class_name, name, desc))
                }
                _ => Err(VmError::InvalidMethodref { index: nat_idx }),
            }
        }
        _ => Err(VmError::InvalidMethodref { index: idx }),
    }
}

/// Count argument slots in a JVM method descriptor like `(ILjava/lang/String;[I)V`.
pub(crate) fn parse_arg_count(descriptor: &str) -> usize {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut count = 0;
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => count += 1,
            '[' => {
                while chars.peek() == Some(&'[') {
                    chars.next();
                }
                if chars.peek() == Some(&'L') {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2 == ';' {
                            break;
                        }
                    }
                } else {
                    chars.next();
                }
                count += 1;
            }
            'L' => {
                for c2 in chars.by_ref() {
                    if c2 == ';' {
                        break;
                    }
                }
                count += 1;
            }
            _ => {}
        }
    }
    count
}

/// Resolve a constant pool Fieldref to (`class_name`, `field_name`, descriptor).
pub(crate) fn resolve_fieldref(
    cp: &[Option<CpEntry>],
    idx: usize,
) -> VmResult<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Fieldref {
            class_index,
            name_and_type_index,
        }) => {
            let class_name = match cp.get(class_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Class { name_index }) => {
                    match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    }
                }
                _ => return Err(VmError::InvalidFieldref { index: idx }),
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    };
                    Ok((class_name, name, desc))
                }
                _ => Err(VmError::InvalidFieldref { index: nat_idx }),
            }
        }
        _ => Err(VmError::InvalidFieldref { index: idx }),
    }
}

/// Resolve a `MethodHandle` CP entry to (`reference_kind`, `class_name`, `method_name`, descriptor).
fn resolve_method_handle(
    cp: &[Option<CpEntry>],
    cp_idx: usize,
) -> VmResult<(u8, String, String, String)> {
    let (kind, ref_idx) = match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::MethodHandle {
            reference_kind,
            reference_index,
        }) => (*reference_kind, reference_index.0 as usize),
        _ => return Err(VmError::InvalidCpIndex { index: cp_idx }),
    };
    let (class_name, method_name, descriptor) = resolve_methodref(cp, ref_idx)?;
    Ok((kind, class_name, method_name, descriptor))
}

/// Resolve a `NameAndType` CP entry to (name, descriptor).
fn resolve_name_and_type(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<(String, String)> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::NameAndType {
            name_index,
            descriptor_index,
        }) => {
            let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => {
                    return Err(VmError::InvalidCpIndex {
                        index: name_index.0 as usize,
                    });
                }
            };
            let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => {
                    return Err(VmError::InvalidCpIndex {
                        index: descriptor_index.0 as usize,
                    });
                }
            };
            Ok((name, desc))
        }
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

/// Resolve a CP String entry to its UTF-8 content. Also handles bare Utf8 entries.
pub(crate) fn resolve_cp_string(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::String { string_index }) => {
            match cp.get(string_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s.clone()),
                _ => Err(VmError::InvalidCpIndex {
                    index: string_index.0 as usize,
                }),
            }
        }
        Some(CpEntry::Utf8(s)) => Ok(s.clone()),
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

/// Parse argument type descriptors from a JVM method descriptor like `(IZLjava/lang/String;)V`.
/// Returns a Vec of single-char type codes: 'I', 'Z', 'L' (for object refs), '[' (for arrays), etc.
pub(crate) fn parse_arg_types(descriptor: &str) -> Vec<char> {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut types = Vec::new();
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => types.push(c),
            '[' => {
                // Skip array dimensions and element type.
                while chars.peek() == Some(&'[') {
                    chars.next();
                }
                if chars.peek() == Some(&'L') {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2 == ';' {
                            break;
                        }
                    }
                } else {
                    chars.next();
                }
                types.push('[');
            }
            'L' => {
                for c2 in chars.by_ref() {
                    if c2 == ';' {
                        break;
                    }
                }
                types.push('L');
            }
            _ => {}
        }
    }
    types
}

/// Index of a named instance field within ctx.fields (non-static only).
/// Return the correct default [`Slot`] for a field with the given JVM descriptor.
///
/// Per JVMS §2.3/2.4: numeric types default to 0, reference/array types to null.
#[inline]
pub(crate) fn default_slot_for_descriptor(desc: &str) -> Slot {
    match desc.chars().next() {
        Some('J') => Slot::Long(0),
        Some('F') => Slot::Float(0.0),
        Some('D') => Slot::Double(0.0),
        Some('L' | '[') => Slot::Reference(None),
        _ => Slot::Int(0), // I, Z, B, C, S
    }
}

/// reference-typed fields (`L…;` / `[…`), which must be `Reference(None)`.
/// Sum a class's instance fields across its full superclass chain.
fn total_instance_field_count(registry: &ClassRegistry, class_name: &str) -> usize {
    let mut count = registry
        .get(class_name)
        .map(|c| c.instance_field_count)
        .unwrap_or(0);
    let mut sc = registry
        .get(class_name)
        .ok()
        .and_then(|c| c.super_class.clone());
    while let Some(ref s) = sc {
        match registry.get(s) {
            Ok(sctx) => {
                count += sctx.instance_field_count;
                sc = sctx.super_class.clone();
            }
            Err(_) => break,
        }
    }
    count
}

/// Allocate a heap exception object for a native-thrown Java exception.
///
/// Native handlers currently surface Java exceptions as class names. Catch
/// blocks need an object reference on the operand stack, so we materialize a
/// minimal heap object of that class before routing through exception-table
/// dispatch.
fn materialize_java_exception_object(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    class_name: &str,
) -> VmResult<u64> {
    registry.ensure_loaded(class_name, loader)?;
    let exc_ref = heap.allocate(
        class_name.to_string(),
        total_instance_field_count(registry, class_name),
    );
    init_object_fields(registry, heap, exc_ref, class_name);
    Ok(exc_ref)
}

/// After allocating an object on the heap, initialize each field slot to the
/// JVM-spec default for its descriptor.
///
/// `heap.allocate` sets all slots to `Slot::Int(0)`, which is wrong for
/// reference-typed fields (`L...;` / `[...]`), which must be `Reference(None)`.
/// This function walks the full class hierarchy (Object-first) and writes the
/// correct default into every slot that differs from `Int(0)`.
fn init_object_fields(
    registry: &ClassRegistry,
    heap: &mut duke_gc::Heap,
    obj_ref: u64,
    class_name: &str,
) {
    // Collect hierarchy: class_name → … → root
    let mut chain: Vec<String> = Vec::new();
    let mut cur = Some(class_name.to_string());
    while let Some(cls) = cur {
        if let Ok(ctx) = registry.get(&cls) {
            let sc = ctx.super_class.clone();
            chain.push(cls);
            cur = sc;
        } else {
            break;
        }
    }
    chain.reverse(); // Object-first

    let mut slot_idx = 0usize;
    for cls in &chain {
        if let Ok(ctx) = registry.get(cls) {
            for field in ctx.fields.iter().filter(|f| !f.is_static) {
                let default = default_slot_for_descriptor(&field.descriptor);
                // Only write non-Int-zero defaults (avoids an unnecessary mut borrow).
                if !matches!(default, Slot::Int(0))
                    && let Ok(obj) = heap.get_mut(obj_ref)
                    && slot_idx < obj.fields.len()
                {
                    obj.fields[slot_idx] = default;
                }
                slot_idx += 1;
            }
        }
    }
}

/// Compute the absolute slot index of a named instance field within a heap
/// object whose class is `target_class` (or any subclass of it).
///
/// JVM `Fieldref` entries name the access class (often a subclass), not
/// necessarily the declaring class. This function walks the full hierarchy
/// from the root (Object) down to `target_class`, searching each class for
/// the field and accumulating the running slot offset as it goes.
///
/// Layout: root fields occupy the lowest-numbered slots; each subclass
/// appends its fields immediately after its superclass's fields.
pub(crate) fn field_slot_idx(
    registry: &ClassRegistry,
    target_class: &str,
    name: &str,
) -> VmResult<usize> {
    // Build chain from target_class up to the root, then reverse for Object-first.
    let mut chain: Vec<String> = Vec::new();
    let mut cur = Some(target_class.to_string());
    while let Some(cls) = cur {
        if let Ok(ctx) = registry.get(&cls) {
            let sc = ctx.super_class.clone();
            chain.push(cls);
            cur = sc;
        } else {
            break;
        }
    }
    chain.reverse();

    let mut slot = 0usize;
    for cls in &chain {
        if let Ok(ctx) = registry.get(cls) {
            let instance_fields: Vec<_> = ctx.fields.iter().filter(|f| !f.is_static).collect();
            if let Some(local_idx) = instance_fields.iter().position(|f| f.name == name) {
                return Ok(slot + local_idx);
            }
            slot += instance_fields.len();
        }
    }

    Err(VmError::InvalidFieldref { index: 0 })
}

/// Index of a named static field within `ctx.static_fields`.
pub(crate) fn static_field_idx(ctx: &ClassContext, name: &str) -> VmResult<usize> {
    ctx.fields
        .iter()
        .filter(|f| f.is_static)
        .position(|f| f.name == name)
        .ok_or(VmError::InvalidFieldref { index: 0 })
}

// ---------------------------------------------------------------------------
// StringBuilder natives
// ---------------------------------------------------------------------------

/// Native: `StringBuilder.<init>()V` — initialise empty buffer.
/// Collect all live Slot values from the interpreter's current execution state.
/// The GC uses these as the root set for reachability analysis.
fn gather_roots(
    frame: &duke_runtime::Frame,
    call_stack: &[CallFrame],
    registry: &ClassRegistry,
) -> Vec<duke_runtime::Slot> {
    let mut roots = Vec::new();
    roots.extend(frame.slots());
    for cf in call_stack {
        roots.extend(cf.frame.slots());
    }
    for ctx in registry.all_classes() {
        roots.extend(ctx.static_fields.iter().copied());
    }
    roots
}

/// Apply GC forwarding pointers to all live interpreter slots after a minor
/// collection. Must be called immediately after `heap.collect()` returns so
/// that stale young-gen references are updated to their new locations.
fn patch_forwarded_slots(
    frame: &mut duke_runtime::Frame,
    call_stack: &mut [CallFrame],
    registry: &mut ClassRegistry,
    heap: &duke_gc::Heap,
) {
    for slot in frame.slots_mut() {
        heap.apply_forward(slot);
    }
    for cf in call_stack.iter_mut() {
        for slot in cf.frame.slots_mut() {
            heap.apply_forward(slot);
        }
    }
    for ctx in registry.all_classes_mut() {
        for slot in &mut ctx.static_fields {
            heap.apply_forward(slot);
        }
    }
}
