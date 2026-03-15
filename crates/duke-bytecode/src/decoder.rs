//! Bytecode decoder: converts raw `Code` attribute bytes into typed [`Instruction`]s.
//!
//! Returns a vector of `(pc, Instruction)` pairs. The `pc` is the byte offset
//! of the instruction within the Code array (as used by branch targets).

use crate::{
    error::{DecodeError, DecodeResult},
    instruction::{ArrayType, Instruction},
    opcodes as op,
};
use duke_classfile::CpIndex;

/// Decode a bytecode sequence from a `Code` attribute into typed instructions.
///
/// # Errors
///
/// Returns [`DecodeError`] if the bytecode is structurally malformed (truncated
/// operands, unknown opcodes, invalid `wide` prefix target, bad array type).
/// Never panics.
pub fn decode(code: &[u8]) -> DecodeResult<Vec<(usize, Instruction)>> {
    let mut cursor = Cursor::new(code);
    let mut instructions = Vec::new();

    while cursor.has_remaining() {
        let pc = cursor.pos;
        let opcode = cursor.read_u8()?;
        let instr = decode_one(&mut cursor, opcode, pc)?;
        instructions.push((pc, instr));
    }

    Ok(instructions)
}

// ---------------------------------------------------------------------------
// Cursor
// ---------------------------------------------------------------------------

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    const fn has_remaining(&self) -> bool {
        self.pos < self.data.len()
    }

    fn read_u8(&mut self) -> DecodeResult<u8> {
        if self.pos >= self.data.len() {
            return Err(DecodeError::UnexpectedEof { pc: self.pos });
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn read_i8(&mut self) -> DecodeResult<i8> {
        #[allow(clippy::cast_possible_wrap)]
        Ok(self.read_u8()? as i8)
    }

    fn read_u16(&mut self) -> DecodeResult<u16> {
        let hi = u16::from(self.read_u8()?);
        let lo = u16::from(self.read_u8()?);
        Ok((hi << 8) | lo)
    }

    fn read_i16(&mut self) -> DecodeResult<i16> {
        #[allow(clippy::cast_possible_wrap)]
        Ok(self.read_u16()? as i16)
    }

    fn read_u32(&mut self) -> DecodeResult<u32> {
        let hi = u32::from(self.read_u16()?);
        let lo = u32::from(self.read_u16()?);
        Ok((hi << 16) | lo)
    }

    fn read_i32(&mut self) -> DecodeResult<i32> {
        #[allow(clippy::cast_possible_wrap)]
        Ok(self.read_u32()? as i32)
    }

    fn read_cp(&mut self) -> DecodeResult<CpIndex> {
        Ok(CpIndex(self.read_u16()?))
    }

    /// Advance to the next 4-byte boundary (relative to the start of the Code array).
    const fn align4(&mut self) {
        let rem = self.pos % 4;
        if rem != 0 {
            self.pos += 4 - rem;
        }
    }
}

// ---------------------------------------------------------------------------
// Core dispatch
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn decode_one(c: &mut Cursor<'_>, opcode: u8, pc: usize) -> DecodeResult<Instruction> {
    let instr = match opcode {
        // -- Constants -------------------------------------------------------
        op::NOP => Instruction::Nop,
        op::ACONST_NULL => Instruction::AconstNull,
        op::ICONST_M1 => Instruction::IconstM1,
        op::ICONST_0 => Instruction::Iconst0,
        op::ICONST_1 => Instruction::Iconst1,
        op::ICONST_2 => Instruction::Iconst2,
        op::ICONST_3 => Instruction::Iconst3,
        op::ICONST_4 => Instruction::Iconst4,
        op::ICONST_5 => Instruction::Iconst5,
        op::LCONST_0 => Instruction::Lconst0,
        op::LCONST_1 => Instruction::Lconst1,
        op::FCONST_0 => Instruction::Fconst0,
        op::FCONST_1 => Instruction::Fconst1,
        op::FCONST_2 => Instruction::Fconst2,
        op::DCONST_0 => Instruction::Dconst0,
        op::DCONST_1 => Instruction::Dconst1,
        op::BIPUSH => Instruction::Bipush(c.read_i8()?),
        op::SIPUSH => Instruction::Sipush(c.read_i16()?),
        op::LDC => Instruction::Ldc(c.read_u8()?),
        op::LDC_W => Instruction::LdcW(c.read_cp()?),
        op::LDC2_W => Instruction::Ldc2W(c.read_cp()?),

        // -- Loads -----------------------------------------------------------
        op::ILOAD => Instruction::Iload(c.read_u8()?),
        op::LLOAD => Instruction::Lload(c.read_u8()?),
        op::FLOAD => Instruction::Fload(c.read_u8()?),
        op::DLOAD => Instruction::Dload(c.read_u8()?),
        op::ALOAD => Instruction::Aload(c.read_u8()?),
        op::ILOAD_0 => Instruction::Iload0,
        op::ILOAD_1 => Instruction::Iload1,
        op::ILOAD_2 => Instruction::Iload2,
        op::ILOAD_3 => Instruction::Iload3,
        op::LLOAD_0 => Instruction::Lload0,
        op::LLOAD_1 => Instruction::Lload1,
        op::LLOAD_2 => Instruction::Lload2,
        op::LLOAD_3 => Instruction::Lload3,
        op::FLOAD_0 => Instruction::Fload0,
        op::FLOAD_1 => Instruction::Fload1,
        op::FLOAD_2 => Instruction::Fload2,
        op::FLOAD_3 => Instruction::Fload3,
        op::DLOAD_0 => Instruction::Dload0,
        op::DLOAD_1 => Instruction::Dload1,
        op::DLOAD_2 => Instruction::Dload2,
        op::DLOAD_3 => Instruction::Dload3,
        op::ALOAD_0 => Instruction::Aload0,
        op::ALOAD_1 => Instruction::Aload1,
        op::ALOAD_2 => Instruction::Aload2,
        op::ALOAD_3 => Instruction::Aload3,
        op::IALOAD => Instruction::Iaload,
        op::LALOAD => Instruction::Laload,
        op::FALOAD => Instruction::Faload,
        op::DALOAD => Instruction::Daload,
        op::AALOAD => Instruction::Aaload,
        op::BALOAD => Instruction::Baload,
        op::CALOAD => Instruction::Caload,
        op::SALOAD => Instruction::Saload,

        // -- Stores ----------------------------------------------------------
        op::ISTORE => Instruction::Istore(c.read_u8()?),
        op::LSTORE => Instruction::Lstore(c.read_u8()?),
        op::FSTORE => Instruction::Fstore(c.read_u8()?),
        op::DSTORE => Instruction::Dstore(c.read_u8()?),
        op::ASTORE => Instruction::Astore(c.read_u8()?),
        op::ISTORE_0 => Instruction::Istore0,
        op::ISTORE_1 => Instruction::Istore1,
        op::ISTORE_2 => Instruction::Istore2,
        op::ISTORE_3 => Instruction::Istore3,
        op::LSTORE_0 => Instruction::Lstore0,
        op::LSTORE_1 => Instruction::Lstore1,
        op::LSTORE_2 => Instruction::Lstore2,
        op::LSTORE_3 => Instruction::Lstore3,
        op::FSTORE_0 => Instruction::Fstore0,
        op::FSTORE_1 => Instruction::Fstore1,
        op::FSTORE_2 => Instruction::Fstore2,
        op::FSTORE_3 => Instruction::Fstore3,
        op::DSTORE_0 => Instruction::Dstore0,
        op::DSTORE_1 => Instruction::Dstore1,
        op::DSTORE_2 => Instruction::Dstore2,
        op::DSTORE_3 => Instruction::Dstore3,
        op::ASTORE_0 => Instruction::Astore0,
        op::ASTORE_1 => Instruction::Astore1,
        op::ASTORE_2 => Instruction::Astore2,
        op::ASTORE_3 => Instruction::Astore3,
        op::IASTORE => Instruction::Iastore,
        op::LASTORE => Instruction::Lastore,
        op::FASTORE => Instruction::Fastore,
        op::DASTORE => Instruction::Dastore,
        op::AASTORE => Instruction::Aastore,
        op::BASTORE => Instruction::Bastore,
        op::CASTORE => Instruction::Castore,
        op::SASTORE => Instruction::Sastore,

        // -- Stack -----------------------------------------------------------
        op::POP => Instruction::Pop,
        op::POP2 => Instruction::Pop2,
        op::DUP => Instruction::Dup,
        op::DUP_X1 => Instruction::DupX1,
        op::DUP_X2 => Instruction::DupX2,
        op::DUP2 => Instruction::Dup2,
        op::DUP2_X1 => Instruction::Dup2X1,
        op::DUP2_X2 => Instruction::Dup2X2,
        op::SWAP => Instruction::Swap,

        // -- Arithmetic ------------------------------------------------------
        op::IADD => Instruction::Iadd,
        op::LADD => Instruction::Ladd,
        op::FADD => Instruction::Fadd,
        op::DADD => Instruction::Dadd,
        op::ISUB => Instruction::Isub,
        op::LSUB => Instruction::Lsub,
        op::FSUB => Instruction::Fsub,
        op::DSUB => Instruction::Dsub,
        op::IMUL => Instruction::Imul,
        op::LMUL => Instruction::Lmul,
        op::FMUL => Instruction::Fmul,
        op::DMUL => Instruction::Dmul,
        op::IDIV => Instruction::Idiv,
        op::LDIV => Instruction::Ldiv,
        op::FDIV => Instruction::Fdiv,
        op::DDIV => Instruction::Ddiv,
        op::IREM => Instruction::Irem,
        op::LREM => Instruction::Lrem,
        op::FREM => Instruction::Frem,
        op::DREM => Instruction::Drem,
        op::INEG => Instruction::Ineg,
        op::LNEG => Instruction::Lneg,
        op::FNEG => Instruction::Fneg,
        op::DNEG => Instruction::Dneg,
        op::ISHL => Instruction::Ishl,
        op::LSHL => Instruction::Lshl,
        op::ISHR => Instruction::Ishr,
        op::LSHR => Instruction::Lshr,
        op::IUSHR => Instruction::Iushr,
        op::LUSHR => Instruction::Lushr,
        op::IAND => Instruction::Iand,
        op::LAND => Instruction::Land,
        op::IOR => Instruction::Ior,
        op::LOR => Instruction::Lor,
        op::IXOR => Instruction::Ixor,
        op::LXOR => Instruction::Lxor,
        op::IINC => Instruction::Iinc {
            index: c.read_u8()?,
            value: c.read_i8()?,
        },

        // -- Conversions -----------------------------------------------------
        op::I2L => Instruction::I2l,
        op::I2F => Instruction::I2f,
        op::I2D => Instruction::I2d,
        op::L2I => Instruction::L2i,
        op::L2F => Instruction::L2f,
        op::L2D => Instruction::L2d,
        op::F2I => Instruction::F2i,
        op::F2L => Instruction::F2l,
        op::F2D => Instruction::F2d,
        op::D2I => Instruction::D2i,
        op::D2L => Instruction::D2l,
        op::D2F => Instruction::D2f,
        op::I2B => Instruction::I2b,
        op::I2C => Instruction::I2c,
        op::I2S => Instruction::I2s,

        // -- Comparisons -----------------------------------------------------
        op::LCMP => Instruction::Lcmp,
        op::FCMPL => Instruction::Fcmpl,
        op::FCMPG => Instruction::Fcmpg,
        op::DCMPL => Instruction::Dcmpl,
        op::DCMPG => Instruction::Dcmpg,

        // -- Branches --------------------------------------------------------
        op::IFEQ => Instruction::Ifeq(c.read_i16()?),
        op::IFNE => Instruction::Ifne(c.read_i16()?),
        op::IFLT => Instruction::Iflt(c.read_i16()?),
        op::IFGE => Instruction::Ifge(c.read_i16()?),
        op::IFGT => Instruction::Ifgt(c.read_i16()?),
        op::IFLE => Instruction::Ifle(c.read_i16()?),
        op::IF_ICMPEQ => Instruction::IfIcmpeq(c.read_i16()?),
        op::IF_ICMPNE => Instruction::IfIcmpne(c.read_i16()?),
        op::IF_ICMPLT => Instruction::IfIcmplt(c.read_i16()?),
        op::IF_ICMPGE => Instruction::IfIcmpge(c.read_i16()?),
        op::IF_ICMPGT => Instruction::IfIcmpgt(c.read_i16()?),
        op::IF_ICMPLE => Instruction::IfIcmple(c.read_i16()?),
        op::IF_ACMPEQ => Instruction::IfAcmpeq(c.read_i16()?),
        op::IF_ACMPNE => Instruction::IfAcmpne(c.read_i16()?),
        op::GOTO => Instruction::Goto(c.read_i16()?),
        op::JSR => Instruction::Jsr(c.read_i16()?),
        op::RET => Instruction::Ret(c.read_u8()?),

        // -- Tableswitch (§6.5 tableswitch) ----------------------------------
        op::TABLESWITCH => {
            // `pc` is the offset of the tableswitch opcode byte.
            // After reading the opcode, cursor is at pc+1.
            // Align to 4-byte boundary from start of code array.
            c.align4();
            let default = c.read_i32()?;
            let low = c.read_i32()?;
            let high = c.read_i32()?;
            if high < low {
                return Err(DecodeError::InvalidTableswitch { pc, low, high });
            }
            // Use i64 to avoid i32 overflow when low is very negative.
            let count_i64 = i64::from(high) - i64::from(low) + 1;
            // Sanity cap: each entry needs 4 bytes; reject if more than remaining data.
            let max_possible = c.data.len().saturating_sub(c.pos) / 4;
            if count_i64 < 0 || usize::try_from(count_i64).unwrap_or(usize::MAX) > max_possible {
                return Err(DecodeError::InvalidTableswitch { pc, low, high });
            }
            let count = usize::try_from(count_i64).unwrap_or(0);
            let mut offsets = Vec::with_capacity(count);
            for _ in 0..count {
                offsets.push(c.read_i32()?);
            }
            Instruction::Tableswitch {
                default,
                low,
                high,
                offsets,
            }
        }

        // -- Lookupswitch (§6.5 lookupswitch) --------------------------------
        op::LOOKUPSWITCH => {
            c.align4();
            let default = c.read_i32()?;
            let npairs = c.read_i32()?;
            if npairs < 0 {
                return Err(DecodeError::InvalidLookupswitch { pc, npairs });
            }
            let remaining_pairs = c.data.len().saturating_sub(c.pos) / 8;
            if usize::try_from(npairs).unwrap_or(usize::MAX) > remaining_pairs {
                return Err(DecodeError::InvalidLookupswitch { pc, npairs });
            }
            let mut pairs = Vec::with_capacity(usize::try_from(npairs).unwrap_or(0));
            for _ in 0..npairs {
                let match_val = c.read_i32()?;
                let offset = c.read_i32()?;
                pairs.push((match_val, offset));
            }
            Instruction::Lookupswitch { default, pairs }
        }

        // -- Returns ---------------------------------------------------------
        op::IRETURN => Instruction::Ireturn,
        op::LRETURN => Instruction::Lreturn,
        op::FRETURN => Instruction::Freturn,
        op::DRETURN => Instruction::Dreturn,
        op::ARETURN => Instruction::Areturn,
        op::RETURN => Instruction::Return,

        // -- Field / method --------------------------------------------------
        op::GETSTATIC => Instruction::Getstatic(c.read_cp()?),
        op::PUTSTATIC => Instruction::Putstatic(c.read_cp()?),
        op::GETFIELD => Instruction::Getfield(c.read_cp()?),
        op::PUTFIELD => Instruction::Putfield(c.read_cp()?),
        op::INVOKEVIRTUAL => Instruction::Invokevirtual(c.read_cp()?),
        op::INVOKESPECIAL => Instruction::Invokespecial(c.read_cp()?),
        op::INVOKESTATIC => Instruction::Invokestatic(c.read_cp()?),
        op::INVOKEINTERFACE => {
            let index = c.read_cp()?;
            let count = c.read_u8()?;
            let zero = c.read_u8()?;
            if zero != 0 {
                return Err(DecodeError::InvalidInvokeinterfaceReserved { pc, reserved: zero });
            }
            Instruction::Invokeinterface { index, count }
        }
        op::INVOKEDYNAMIC => {
            let index = c.read_cp()?;
            let zero1 = c.read_u8()?;
            let zero2 = c.read_u8()?;
            if zero1 != 0 || zero2 != 0 {
                return Err(DecodeError::InvalidInvokedynamicReserved {
                    pc,
                    reserved1: zero1,
                    reserved2: zero2,
                });
            }
            Instruction::Invokedynamic(index)
        }

        // -- Object / array --------------------------------------------------
        op::NEW => Instruction::New(c.read_cp()?),
        op::NEWARRAY => {
            let t = c.read_u8()?;
            let array_type = ArrayType::from_u8(t)
                .ok_or(DecodeError::InvalidNewarrayType { pc, type_code: t })?;
            Instruction::Newarray(array_type)
        }
        op::ANEWARRAY => Instruction::Anewarray(c.read_cp()?),
        op::ARRAYLENGTH => Instruction::Arraylength,
        op::ATHROW => Instruction::Athrow,
        op::CHECKCAST => Instruction::Checkcast(c.read_cp()?),
        op::INSTANCEOF => Instruction::Instanceof(c.read_cp()?),
        op::MONITORENTER => Instruction::Monitorenter,
        op::MONITOREXIT => Instruction::Monitorexit,

        // -- Wide prefix (§6.5 wide) -----------------------------------------
        op::WIDE => decode_wide(c, pc)?,

        // -- Extended --------------------------------------------------------
        op::MULTIANEWARRAY => {
            let index = c.read_cp()?;
            let dimensions = c.read_u8()?;
            Instruction::Multianewarray { index, dimensions }
        }
        op::IFNULL => Instruction::Ifnull(c.read_i16()?),
        op::IFNONNULL => Instruction::Ifnonnull(c.read_i16()?),
        op::GOTO_W => Instruction::GotoW(c.read_i32()?),
        op::JSR_W => Instruction::JsrW(c.read_i32()?),

        // -- Unknown ---------------------------------------------------------
        other => return Err(DecodeError::UnknownOpcode { pc, opcode: other }),
    };
    Ok(instr)
}

// ---------------------------------------------------------------------------
// Wide prefix handler
// ---------------------------------------------------------------------------

fn decode_wide(c: &mut Cursor<'_>, pc: usize) -> DecodeResult<Instruction> {
    let opcode = c.read_u8()?;
    let instr = match opcode {
        op::ILOAD => Instruction::IloadW(c.read_u16()?),
        op::LLOAD => Instruction::LloadW(c.read_u16()?),
        op::FLOAD => Instruction::FloadW(c.read_u16()?),
        op::DLOAD => Instruction::DloadW(c.read_u16()?),
        op::ALOAD => Instruction::AloadW(c.read_u16()?),
        op::ISTORE => Instruction::IstoreW(c.read_u16()?),
        op::LSTORE => Instruction::LstoreW(c.read_u16()?),
        op::FSTORE => Instruction::FstoreW(c.read_u16()?),
        op::DSTORE => Instruction::DstoreW(c.read_u16()?),
        op::ASTORE => Instruction::AstoreW(c.read_u16()?),
        op::RET => Instruction::RetW(c.read_u16()?),
        op::IINC => Instruction::IincW {
            index: c.read_u16()?,
            value: c.read_i16()?,
        },
        other => return Err(DecodeError::InvalidWideTarget { pc, opcode: other }),
    };
    Ok(instr)
}
