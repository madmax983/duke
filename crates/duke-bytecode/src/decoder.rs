//! Bytecode decoder: converts raw `Code` attribute bytes into typed [`Instruction`]s.
//!
//! Returns a vector of `(pc, Instruction)` pairs. The `pc` is the byte offset
//! of the instruction within the Code array (as used by branch targets).

use crate::{
    error::{DecodeError, Result},
    instruction::{ArrayType, Instruction},
    opcodes as op,
};
use duke_classfile::CpIndex;

/// Decode a bytecode sequence from a `Code` attribute into typed instructions.
///
/// This performs the first pass over a raw byte stream, resolving variable-length
/// instruction operands into a typed [`Instruction`] enum paired with its original
/// byte offset (`pc`).
///
/// # Errors
///
/// Returns [`DecodeError`] if the bytecode is structurally malformed (truncated
/// operands, unknown opcodes, invalid `wide` prefix target, bad array type).
/// Never panics.
///
/// # Examples
///
/// ```
/// use duke_bytecode::{decode, Instruction};
///
/// // iconst_1 (0x04), ireturn (0xAC)
/// let raw_code = [0x04, 0xAC];
///
/// let decoded = decode(&raw_code).unwrap();
/// assert_eq!(decoded.len(), 2);
/// assert_eq!(decoded[0], (0, Instruction::Iconst1));
/// assert_eq!(decoded[1], (1, Instruction::Ireturn));
/// ```
pub fn decode(code: &[u8]) -> Result<Vec<(usize, Instruction)>> {
    let mut cursor = Cursor::new(code);
    // ⚡ Bolt: Pre-allocate capacity for the decoded instructions vector to avoid
    // multiple reallocations during parsing. A heuristic of 3 bytes per instruction
    // provides a good balance between memory overhead and allocation avoidance.
    let mut instructions = Vec::with_capacity(code.len() / 3);

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

    fn read_u8(&mut self) -> Result<u8> {
        if self.pos >= self.data.len() {
            return Err(crate::Error::Decode(DecodeError::UnexpectedEof {
                pc: self.pos,
            }));
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn read_i8(&mut self) -> Result<i8> {
        Ok(self.read_u8()?.cast_signed())
    }

    fn read_u16(&mut self) -> Result<u16> {
        if self.pos + 2 > self.data.len() {
            return Err(crate::Error::Decode(DecodeError::UnexpectedEof {
                pc: self.pos,
            }));
        }
        let bytes = [self.data[self.pos], self.data[self.pos + 1]];
        self.pos += 2;
        Ok(u16::from_be_bytes(bytes))
    }

    fn read_i16(&mut self) -> Result<i16> {
        Ok(self.read_u16()?.cast_signed())
    }

    fn read_u32(&mut self) -> Result<u32> {
        if self.pos + 4 > self.data.len() {
            return Err(crate::Error::Decode(DecodeError::UnexpectedEof {
                pc: self.pos,
            }));
        }
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ];
        self.pos += 4;
        Ok(u32::from_be_bytes(bytes))
    }

    fn read_i32(&mut self) -> Result<i32> {
        Ok(self.read_u32()?.cast_signed())
    }

    fn read_cp(&mut self) -> Result<CpIndex> {
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
fn decode_one(c: &mut Cursor<'_>, opcode: u8, pc: usize) -> Result<Instruction> {
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
        op::TABLESWITCH => decode_tableswitch(c, pc)?,

        // -- Lookupswitch (§6.5 lookupswitch) --------------------------------
        op::LOOKUPSWITCH => decode_lookupswitch(c, pc)?,

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
                return Err(crate::Error::Decode(
                    DecodeError::InvalidInvokeinterfaceReserved { pc, reserved: zero },
                ));
            }
            Instruction::Invokeinterface { index, count }
        }
        op::INVOKEDYNAMIC => {
            let index = c.read_cp()?;
            let zero1 = c.read_u8()?;
            let zero2 = c.read_u8()?;
            if zero1 != 0 || zero2 != 0 {
                return Err(crate::Error::Decode(
                    DecodeError::InvalidInvokedynamicReserved {
                        pc,
                        reserved1: zero1,
                        reserved2: zero2,
                    },
                ));
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
        other => {
            return Err(crate::Error::Decode(DecodeError::UnknownOpcode {
                pc,
                opcode: other,
            }));
        }
    };
    Ok(instr)
}

// ---------------------------------------------------------------------------
fn decode_tableswitch(c: &mut Cursor<'_>, pc: usize) -> Result<Instruction> {
    // `pc` is the offset of the tableswitch opcode byte.
    // After reading the opcode, cursor is at pc+1.
    // Align to 4-byte boundary from start of code array.
    c.align4();
    let default = c.read_i32()?;
    let low = c.read_i32()?;
    let high = c.read_i32()?;
    if high < low {
        return Err(crate::Error::Decode(DecodeError::InvalidTableswitch {
            pc,
            low,
            high,
        }));
    }
    // Use i64 to avoid i32 overflow when low is very negative.
    let count_i64 = i64::from(high) - i64::from(low) + 1;
    // Sanity cap: each entry needs 4 bytes; reject if more than remaining data.
    let max_possible = c.data.len().saturating_sub(c.pos) / 4;
    if count_i64 < 0 || usize::try_from(count_i64).unwrap_or(usize::MAX) > max_possible {
        return Err(crate::Error::Decode(DecodeError::InvalidTableswitch {
            pc,
            low,
            high,
        }));
    }
    let count = usize::try_from(count_i64).unwrap_or(0);
    let mut offsets = Vec::with_capacity(count.min(c.data.len().saturating_sub(c.pos) / 4));
    for _ in 0..count {
        offsets.push(c.read_i32()?);
    }
    Ok(Instruction::Tableswitch {
        default,
        low,
        high,
        offsets,
    })
}

fn decode_lookupswitch(c: &mut Cursor<'_>, pc: usize) -> Result<Instruction> {
    c.align4();
    let default = c.read_i32()?;
    let npairs = c.read_i32()?;
    if npairs < 0 {
        return Err(crate::Error::Decode(DecodeError::InvalidLookupswitch {
            pc,
            npairs,
        }));
    }
    let remaining_pairs = c.data.len().saturating_sub(c.pos) / 8;
    if usize::try_from(npairs).unwrap_or(usize::MAX) > remaining_pairs {
        return Err(crate::Error::Decode(DecodeError::InvalidLookupswitch {
            pc,
            npairs,
        }));
    }
    let npairs_usize = usize::try_from(npairs).unwrap_or(0);
    let mut pairs = Vec::with_capacity(npairs_usize.min(c.data.len().saturating_sub(c.pos) / 8));
    for _ in 0..npairs_usize {
        let match_val = c.read_i32()?;
        let offset = c.read_i32()?;
        pairs.push((match_val, offset));
    }
    Ok(Instruction::Lookupswitch { default, pairs })
}

// Wide prefix handler
// ---------------------------------------------------------------------------

fn decode_wide(c: &mut Cursor<'_>, pc: usize) -> Result<Instruction> {
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
        other => {
            return Err(crate::Error::Decode(DecodeError::InvalidWideTarget {
                pc,
                opcode: other,
            }));
        }
    };
    Ok(instr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_truncated_operands() {
        // Sipush needs 2 bytes, only 1 provided
        let code = [op::SIPUSH, 0x01];
        let err = decode(&code).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::Decode(DecodeError::UnexpectedEof { pc: 1 })
        ));
    }

    #[test]
    fn test_decoder_unknown_opcode() {
        let code = [0xFF];
        let err = decode(&code).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::Decode(DecodeError::UnknownOpcode {
                pc: 0,
                opcode: 0xFF
            })
        ));
    }

    #[test]
    fn test_decoder_invalid_wide_prefix() {
        // Wide followed by an invalid opcode (e.g. NOP)
        let code = [op::WIDE, op::NOP];
        let err = decode(&code).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::Decode(DecodeError::InvalidWideTarget {
                pc: 0,
                opcode: op::NOP
            })
        ));
    }

    #[test]
    fn test_decoder_spot_checks() {
        // Iload2, Fload0, Saload, Dstore3, Swap, Newarray(Int)
        let code = vec![0x1C, 0x22, 0x35, 0x4A, 0x5F, 0xBC, 0x0A];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 6);
        assert_eq!(instrs[0].1, Instruction::Iload2);
        assert_eq!(instrs[1].1, Instruction::Fload0);
        assert_eq!(instrs[2].1, Instruction::Saload);
        assert_eq!(instrs[3].1, Instruction::Dstore3);
        assert_eq!(instrs[4].1, Instruction::Swap);
        assert_eq!(instrs[5].1, Instruction::Newarray(ArrayType::Int));
    }

    #[test]
    fn test_decoder_spot_checks2() {
        // Astore3, Athrow
        let code = vec![0x4E, 0xBF];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].1, Instruction::Astore3);
        assert_eq!(instrs[1].1, Instruction::Athrow);
    }

    #[test]
    fn test_decoder_spot_checks3() {
        // Sastore, Goto
        let code = vec![0x56, 0xA7, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].1, Instruction::Sastore);
        assert_eq!(instrs[1].1, Instruction::Goto(0x0102));
    }

    #[test]
    fn test_decoder_spot_checks4() {
        // Aload3, Caload, Iastore, Pop2, Iinc(index=1, value=2)
        let code = vec![0x2D, 0x34, 0x4F, 0x58, 0x84, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 5);
        assert_eq!(instrs[0].1, Instruction::Aload3);
        assert_eq!(instrs[1].1, Instruction::Caload);
        assert_eq!(instrs[2].1, Instruction::Iastore);
        assert_eq!(instrs[3].1, Instruction::Pop2);
        assert_eq!(instrs[4].1, Instruction::Iinc { index: 1, value: 2 });
    }

    #[test]
    fn test_decoder_spot_checks5() {
        // Fstore1
        let code = vec![0x44];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::Fstore1);
    }

    #[test]
    fn test_decoder_coverage_additional() {
        let code = vec![
            op::CASTORE,
            op::SASTORE,
            op::RET,
            0x01,
            op::PUTFIELD,
            0x00,
            0x01,
            op::MULTIANEWARRAY,
            0x00,
            0x02,
            0x03,
        ];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 5);
        assert_eq!(instrs[0].1, Instruction::Castore);
        assert_eq!(instrs[1].1, Instruction::Sastore);
        assert_eq!(instrs[2].1, Instruction::Ret(1));
        assert_eq!(
            instrs[3].1,
            Instruction::Putfield(duke_classfile::CpIndex(1))
        );
        assert_eq!(
            instrs[4].1,
            Instruction::Multianewarray {
                index: duke_classfile::CpIndex(2),
                dimensions: 3
            }
        );
    }

    #[test]
    fn test_decoder_spot_checks6() {
        // Iconst5, Dload3
        let code = vec![0x08, 0x29];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].1, Instruction::Iconst5);
        assert_eq!(instrs[1].1, Instruction::Dload3);
    }

    #[test]
    fn test_decoder_spot_checks7() {
        // Iconst4, Ifgt
        let code = vec![0x07, 0x9D, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].1, Instruction::Iconst4);
        assert_eq!(instrs[1].1, Instruction::Ifgt(0x0102));
    }

    #[test]
    fn test_decoder_spot_checks8() {
        // IfIcmpeq
        let code = vec![0x9F, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::IfIcmpeq(0x0102));
    }

    #[test]
    fn test_decoder_spot_checks9() {
        // Astore1, Irem
        let code = vec![0x4C, 0x70];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].1, Instruction::Astore1);
        assert_eq!(instrs[1].1, Instruction::Irem);
    }

    #[test]
    fn test_decoder_spot_checks10() {
        // Lsub, IfIcmple, Checkcast
        let code = vec![0x65, 0xA4, 0x01, 0x02, 0xC0, 0x03, 0x04];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 3);
        assert_eq!(instrs[0].1, Instruction::Lsub);
        assert_eq!(instrs[1].1, Instruction::IfIcmple(0x0102));
        assert_eq!(instrs[2].1, Instruction::Checkcast(CpIndex(0x0304)));
    }

    #[test]
    fn test_decoder_spot_checks11() {
        // IconstM1, Astore, Ishr
        let code = vec![0x02, 0x3A, 0x01, 0x7A];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 3);
        assert_eq!(instrs[0].1, Instruction::IconstM1);
        assert_eq!(instrs[1].1, Instruction::Astore(0x01));
        assert_eq!(instrs[2].1, Instruction::Ishr);
    }

    #[test]
    fn test_decoder_spot_checks12() {
        // Lstore0, Lshr
        let code = vec![0x3F, 0x7B];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].1, Instruction::Lstore0);
        assert_eq!(instrs[1].1, Instruction::Lshr);
    }

    #[test]
    fn test_decoder_spot_checks13() {
        // Dload2
        let code = vec![0x28];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::Dload2);
    }

    #[test]
    fn test_decoder_spot_checks14() {
        // Dload1, I2l, Dcmpl, Monitorexit
        let code = vec![0x27, 0x85, 0x97, 0xC3];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 4);
        assert_eq!(instrs[0].1, Instruction::Dload1);
        assert_eq!(instrs[1].1, Instruction::I2l);
        assert_eq!(instrs[2].1, Instruction::Dcmpl);
        assert_eq!(instrs[3].1, Instruction::Monitorexit);
    }

    #[test]
    fn test_decoder_spot_checks15() {
        // Dload0
        let code = vec![0x26];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::Dload0);
    }

    #[test]
    fn test_decoder_spot_checks16() {
        // Iconst2, LdcW, Ret, Anewarray
        let code = vec![0x05, 0x13, 0x01, 0x02, 0xA9, 0x01, 0xBD, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 4);
        assert_eq!(instrs[0].1, Instruction::Iconst2);
        assert_eq!(instrs[1].1, Instruction::LdcW(CpIndex(0x0102)));
        assert_eq!(instrs[2].1, Instruction::Ret(0x01));
        assert_eq!(instrs[3].1, Instruction::Anewarray(CpIndex(0x0102)));
    }

    #[test]
    fn test_decoder_spot_checks17() {
        // New
        let code = vec![0xBB, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::New(CpIndex(0x0102)));
    }

    #[test]
    fn test_decoder_spot_checks18() {
        // Ior, Lcmp
        let code = vec![0x80, 0x94];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].1, Instruction::Ior);
        assert_eq!(instrs[1].1, Instruction::Lcmp);
    }

    #[test]
    fn test_decoder_spot_checks19() {
        // Lload2, Lload3, Istore0
        let code = vec![0x20, 0x21, 0x3B];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 3);
        assert_eq!(instrs[0].1, Instruction::Lload2);
        assert_eq!(instrs[1].1, Instruction::Lload3);
        assert_eq!(instrs[2].1, Instruction::Istore0);
    }

    #[test]
    fn test_decoder_spot_checks20() {
        // Drem, IfIcmpne
        let code = vec![0x73, 0xA0, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].1, Instruction::Drem);
        assert_eq!(instrs[1].1, Instruction::IfIcmpne(0x0102));
    }

    #[test]
    fn test_decoder_spot_checks21() {
        // Dstore1, Dcmpg
        let code = vec![0x48, 0x98];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].1, Instruction::Dstore1);
        assert_eq!(instrs[1].1, Instruction::Dcmpg);
    }

    #[test]
    fn test_decoder_spot_checks22() {
        // Pop, Iushr
        let code = vec![0x57, 0x7C];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].1, Instruction::Pop);
        assert_eq!(instrs[1].1, Instruction::Iushr);
    }

    #[test]
    fn test_decoder_spot_checks23() {
        // Aload2, Lstore2, Ifnonnull
        let code = vec![0x2C, 0x41, 0xC7, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 3);
        assert_eq!(instrs[0].1, Instruction::Aload2);
        assert_eq!(instrs[1].1, Instruction::Lstore2);
        assert_eq!(instrs[2].1, Instruction::Ifnonnull(0x0102));
    }

    #[test]
    fn test_decoder_spot_checks24() {
        // Istore2, Astore2
        let code = vec![0x3D, 0x4D];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].1, Instruction::Istore2);
        assert_eq!(instrs[1].1, Instruction::Astore2);
    }

    #[test]
    fn test_decoder_spot_checks25() {
        // Ldiv
        let code = vec![0x6D];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::Ldiv);
    }

    #[test]
    fn test_decoder_spot_checks26() {
        // Fastore
        let code = vec![0x51];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::Fastore);
    }

    #[test]
    fn test_decoder_spot_checks27() {
        // Ifne
        let code = vec![0x9A, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::Ifne(0x0102));
    }

    #[test]
    fn test_decoder_spot_checks28() {
        // Isub, Idiv
        let code = vec![0x64, 0x6C];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].1, Instruction::Isub);
        assert_eq!(instrs[1].1, Instruction::Idiv);
    }

    #[test]
    fn test_decoder_spot_checks29() {
        // Dload, Iload3, Lneg, Monitorenter
        let code = vec![0x18, 0x01, 0x1D, 0x75, 0xC2];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 4);
        assert_eq!(instrs[0].1, Instruction::Dload(0x01));
        assert_eq!(instrs[1].1, Instruction::Iload3);
        assert_eq!(instrs[2].1, Instruction::Lneg);
        assert_eq!(instrs[3].1, Instruction::Monitorenter);
    }

    #[test]
    fn test_decoder_spot_checks30() {
        // Lload1, Fmul, Lrem, Iand
        let code = vec![0x1F, 0x6A, 0x71, 0x7E];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 4);
        assert_eq!(instrs[0].1, Instruction::Lload1);
        assert_eq!(instrs[1].1, Instruction::Fmul);
        assert_eq!(instrs[2].1, Instruction::Lrem);
        assert_eq!(instrs[3].1, Instruction::Iand);
    }

    #[test]
    fn test_decoder_spot_checks31() {
        // Ldc2W, Aload1
        let code = vec![0x14, 0x01, 0x02, 0x2B];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].1, Instruction::Ldc2W(CpIndex(0x0102)));
        assert_eq!(instrs[1].1, Instruction::Aload1);
    }

    #[test]
    fn test_decoder_spot_checks32() {
        // Faload, Ineg, Instanceof
        let code = vec![0x30, 0x74, 0xC1, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 3);
        assert_eq!(instrs[0].1, Instruction::Faload);
        assert_eq!(instrs[1].1, Instruction::Ineg);
        assert_eq!(instrs[2].1, Instruction::Instanceof(CpIndex(0x0102)));
    }

    #[test]
    fn test_decoder_spot_checks33() {
        // Dstore
        let code = vec![0x39, 0x01];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::Dstore(0x01));
    }

    #[test]
    fn test_decoder_spot_checks34() {
        // Fconst2
        let code = vec![0x0D];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::Fconst2);
    }

    #[test]
    fn test_decoder_spot_checks35() {
        // Lor
        let code = vec![0x81];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::Lor);
    }

    #[test]
    fn test_decoder_invalid_lookupswitch_npairs() {
        // npairs < 0
        let code = vec![
            op::LOOKUPSWITCH,
            0x00,
            0x00,
            0x00, // padding
            0x00,
            0x00,
            0x00,
            0x05, // default
            0xFF,
            0xFF,
            0xFF,
            0xFF, // npairs = -1
        ];
        let err = decode(&code).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::Decode(DecodeError::InvalidLookupswitch { pc: 0, npairs: -1 })
        ));
    }

    #[test]
    fn test_decoder_wide_instructions() {
        // wide lload
        let code = vec![0xC4, 0x16, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::LloadW(0x0102));

        // wide fload
        let code = vec![0xC4, 0x17, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::FloadW(0x0102));

        // wide dload
        let code = vec![0xC4, 0x18, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::DloadW(0x0102));

        // wide aload
        let code = vec![0xC4, 0x19, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::AloadW(0x0102));

        // wide istore
        let code = vec![0xC4, 0x36, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::IstoreW(0x0102));

        // wide lstore
        let code = vec![0xC4, 0x37, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::LstoreW(0x0102));

        // wide fstore
        let code = vec![0xC4, 0x38, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::FstoreW(0x0102));

        // wide dstore
        let code = vec![0xC4, 0x39, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::DstoreW(0x0102));

        // wide astore
        let code = vec![0xC4, 0x3A, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::AstoreW(0x0102));

        // wide ret
        let code = vec![0xC4, 0xA9, 0x01, 0x02];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].1, Instruction::RetW(0x0102));

        // wide iinc
        let code = vec![0xC4, 0x84, 0x01, 0x02, 0xFF, 0xFE];
        let instrs = decode(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(
            instrs[0].1,
            Instruction::IincW {
                index: 0x0102,
                value: -2
            }
        );
    }
    #[test]
    fn test_decoder_bad_array_type() {
        let code = [op::NEWARRAY, 0xFF]; // 0xFF is not a valid ArrayType code
        let err = decode(&code).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::Decode(DecodeError::InvalidNewarrayType {
                pc: 0,
                type_code: 0xFF
            })
        ));
    }

    #[test]
    fn test_decoder_invalid_tableswitch() {
        // high < low
        // tableswitch, padding(3), default(4), low(4), high(4)
        let code = [
            op::TABLESWITCH,
            0,
            0,
            0,
            0,
            0,
            0,
            0, // default
            0,
            0,
            0,
            5, // low = 5
            0,
            0,
            0,
            1, // high = 1
        ];
        let err = decode(&code).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::Decode(DecodeError::InvalidTableswitch {
                pc: 0,
                low: 5,
                high: 1
            })
        ));
    }

    #[test]
    fn test_decoder_invalid_tableswitch_high_less_than_low() {
        // tableswitch, padding(3), default(4), low(4), high(4)
        let code = [
            op::TABLESWITCH,
            0,
            0,
            0,
            0,
            0,
            0,
            0, // default
            0,
            0,
            0,
            5, // low = 5
            0,
            0,
            0,
            1, // high = 1
        ];
        let err = decode(&code).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::Decode(DecodeError::InvalidTableswitch {
                pc: 0,
                low: 5,
                high: 1
            })
        ));
    }

    #[test]
    fn test_decoder_tableswitch_too_many_entries() {
        // count = high - low + 1 = 10 - 0 + 1 = 11, but only 4 bytes of offsets provided.
        let code = [
            op::TABLESWITCH,
            0,
            0,
            0, // padding
            0,
            0,
            0,
            0, // default
            0,
            0,
            0,
            0, // low = 0
            0,
            0,
            0,
            10, // high = 10
            0,
            0,
            0,
            0, // 1 entry provided, need 11
        ];
        let err = decode(&code).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::Decode(DecodeError::InvalidTableswitch {
                pc: 0,
                low: 0,
                high: 10
            })
        ));
    }

    #[test]
    fn test_decoder_lookupswitch_too_many_pairs() {
        // npairs = 10, but only 8 bytes of pairs provided.
        let code = [
            op::LOOKUPSWITCH,
            0,
            0,
            0, // padding
            0,
            0,
            0,
            0, // default
            0,
            0,
            0,
            10, // npairs = 10
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0, // 1 pair provided, need 10
        ];
        let err = decode(&code).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::Decode(DecodeError::InvalidLookupswitch { pc: 0, npairs: 10 })
        ));
    }

    #[test]
    fn test_decoder_cursor_read_operations() {
        let mut cursor = Cursor::new(&[0x01, 0xFF, 0x00, 0x02, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00]);

        assert!(cursor.has_remaining());
        assert_eq!(cursor.read_u8().unwrap(), 0x01);
        assert_eq!(cursor.read_i8().unwrap(), -1);
        assert_eq!(cursor.read_u16().unwrap(), 0x0002);
        assert_eq!(cursor.read_i32().unwrap(), -1);

        // At position 8. Buffer is length 10. `has_remaining` should be true.
        assert!(cursor.has_remaining());
        let _ = cursor.read_u16().unwrap();
        // Now position 10.
        assert!(!cursor.has_remaining());

        // Out of bounds read.
        assert!(matches!(
            cursor.read_u8(),
            Err(crate::Error::Decode(DecodeError::UnexpectedEof { pc: 10 }))
        ));

        // align4 test
        let mut cursor2 = Cursor::new(&[0x00, 0x01]);
        assert_eq!(cursor2.read_u8().unwrap(), 0x00);
        cursor2.align4();
        // Since pos is 1, rem is 1. align4 adds 3, making pos 4.
        assert_eq!(cursor2.pos, 4);

        // cp read
        let mut cursor3 = Cursor::new(&[0x00, 0x42]);
        assert_eq!(cursor3.read_cp().unwrap().0, 0x42);

        // i16 test
        let mut cursor4 = Cursor::new(&[0xFF, 0xFF]);
        assert_eq!(cursor4.read_i16().unwrap(), -1);
    }
    #[test]
    fn test_cursor_read_operations() {
        let mut cursor = Cursor::new(&[0x01, 0xFF, 0x00, 0x02, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00]);

        assert!(cursor.has_remaining());
        assert_eq!(cursor.read_u8().unwrap(), 0x01);
        assert_eq!(cursor.read_i8().unwrap(), -1);
        assert_eq!(cursor.read_u16().unwrap(), 0x0002);
        assert_eq!(cursor.read_i32().unwrap(), -1);

        assert!(cursor.has_remaining());
        let _ = cursor.read_u16().unwrap();
        assert!(!cursor.has_remaining());

        let err = cursor.read_u8().unwrap_err();
        assert!(matches!(
            err,
            crate::Error::Decode(DecodeError::UnexpectedEof { pc: 10 })
        ));

        let mut cursor2 = Cursor::new(&[0x00, 0x01]);
        assert_eq!(cursor2.read_u8().unwrap(), 0x00);
        cursor2.align4();
        assert_eq!(cursor2.pos, 4);

        let mut cursor3 = Cursor::new(&[0x00, 0x42]);
        assert_eq!(cursor3.read_cp().unwrap().0, 0x42);

        let mut cursor4 = Cursor::new(&[0xFF, 0xFF]);
        assert_eq!(cursor4.read_i16().unwrap(), -1);

        let mut cursor5 = Cursor::new(&[0x01]);
        assert!(matches!(
            cursor5.read_u16().unwrap_err(),
            crate::Error::Decode(_)
        ));

        let mut cursor6 = Cursor::new(&[0x01, 0x02, 0x03]);
        assert!(matches!(
            cursor6.read_u32().unwrap_err(),
            crate::Error::Decode(_)
        ));
    }
}
