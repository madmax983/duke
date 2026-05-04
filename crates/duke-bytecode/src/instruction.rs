//! Typed JVM instruction representation.
//!
//! Each [`Instruction`] variant carries exactly the operands decoded from
//! the bytecode stream. The variant names match the JVM spec opcode names
//! (`PascalCase`). Wide-prefixed forms carry a `u16` local index instead of `u8`.

use duke_classfile::CpIndex;

/// Array type codes used by the `newarray` instruction (JVM spec §6.5).
///
/// # Examples
///
/// ```
/// use duke_bytecode::ArrayType;
///
/// let int_array = ArrayType::from_u8(10).unwrap();
/// assert_eq!(int_array, ArrayType::Int);
///
/// assert_eq!(ArrayType::from_u8(99), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayType {
    /// Boolean array
    Boolean = 4,
    /// Char array
    Char = 5,
    /// Float array
    Float = 6,
    /// Double array
    Double = 7,
    /// Byte array
    Byte = 8,
    /// Short array
    Short = 9,
    /// Int array
    Int = 10,
    /// Long array
    Long = 11,
}

impl ArrayType {
    /// Returns the array type from a raw byte value.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            4 => Self::Boolean,
            5 => Self::Char,
            6 => Self::Float,
            7 => Self::Double,
            8 => Self::Byte,
            9 => Self::Short,
            10 => Self::Int,
            11 => Self::Long,
            _ => return None,
        })
    }
}

/// A single decoded JVM instruction with its operands.
///
/// Wide-prefixed variants (e.g., `IloadW`) are represented as distinct variants
/// so callers can exhaustively match without needing to track a separate `wide` flag.
///
/// # Examples
///
/// ```
/// use duke_bytecode::Instruction;
///
/// let instr = Instruction::Iconst1;
/// assert_eq!(instr.mnemonic(), "iconst_1");
///
/// let load = Instruction::Iload(5);
/// if let Instruction::Iload(index) = load {
///     assert_eq!(index, 5);
/// }
/// ```
///
/// # Examples
///
/// ```
/// use duke_bytecode::Instruction;
/// use duke_classfile::CpIndex;
///
/// let i_load = Instruction::Iload(4);
/// assert_eq!(i_load.mnemonic(), "iload");
///
/// let get_field = Instruction::Getfield(CpIndex(42));
/// assert_eq!(get_field.mnemonic(), "getfield");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]

pub enum Instruction {
    // -----------------------------------------------------------------------
    // Constants
    // -----------------------------------------------------------------------
    /// JVM `nop` instruction.
    Nop,
    /// JVM `aconstnull` instruction.
    AconstNull,
    /// JVM `iconstm1` instruction.
    IconstM1,
    /// JVM `iconst0` instruction.
    Iconst0,
    /// JVM `iconst1` instruction.
    Iconst1,
    /// JVM `iconst2` instruction.
    Iconst2,
    /// JVM `iconst3` instruction.
    Iconst3,
    /// JVM `iconst4` instruction.
    Iconst4,
    /// JVM `iconst5` instruction.
    Iconst5,
    /// JVM `lconst0` instruction.
    Lconst0,
    /// JVM `lconst1` instruction.
    Lconst1,
    /// JVM `fconst0` instruction.
    Fconst0,
    /// JVM `fconst1` instruction.
    Fconst1,
    /// JVM `fconst2` instruction.
    Fconst2,
    /// JVM `dconst0` instruction.
    Dconst0,
    /// JVM `dconst1` instruction.
    Dconst1,
    /// JVM `bipush` instruction.
    Bipush(i8),
    /// JVM `sipush` instruction.
    Sipush(i16),
    /// JVM `ldc` instruction.
    Ldc(u8),
    /// JVM `ldcw` instruction.
    LdcW(CpIndex),
    /// JVM `ldc2w` instruction.
    Ldc2W(CpIndex),

    // -----------------------------------------------------------------------
    // Loads
    // -----------------------------------------------------------------------
    /// JVM `iload` instruction.
    Iload(u8),
    /// JVM `lload` instruction.
    Lload(u8),
    /// JVM `fload` instruction.
    Fload(u8),
    /// JVM `dload` instruction.
    Dload(u8),
    /// JVM `aload` instruction.
    Aload(u8),
    /// JVM `iload0` instruction.
    Iload0,
    /// JVM `iload1` instruction.
    Iload1,
    /// JVM `iload2` instruction.
    Iload2,
    /// JVM `iload3` instruction.
    Iload3,
    /// JVM `lload0` instruction.
    Lload0,
    /// JVM `lload1` instruction.
    Lload1,
    /// JVM `lload2` instruction.
    Lload2,
    /// JVM `lload3` instruction.
    Lload3,
    /// JVM `fload0` instruction.
    Fload0,
    /// JVM `fload1` instruction.
    Fload1,
    /// JVM `fload2` instruction.
    Fload2,
    /// JVM `fload3` instruction.
    Fload3,
    /// JVM `dload0` instruction.
    Dload0,
    /// JVM `dload1` instruction.
    Dload1,
    /// JVM `dload2` instruction.
    Dload2,
    /// JVM `dload3` instruction.
    Dload3,
    /// JVM `aload0` instruction.
    Aload0,
    /// JVM `aload1` instruction.
    Aload1,
    /// JVM `aload2` instruction.
    Aload2,
    /// JVM `aload3` instruction.
    Aload3,
    /// JVM `iaload` instruction.
    Iaload,
    /// JVM `laload` instruction.
    Laload,
    /// JVM `faload` instruction.
    Faload,
    /// JVM `daload` instruction.
    Daload,
    /// JVM `aaload` instruction.
    Aaload,
    /// JVM `baload` instruction.
    Baload,
    /// JVM `caload` instruction.
    Caload,
    /// JVM `saload` instruction.
    Saload,

    // -----------------------------------------------------------------------
    // Stores
    // -----------------------------------------------------------------------
    /// JVM `istore` instruction.
    Istore(u8),
    /// JVM `lstore` instruction.
    Lstore(u8),
    /// JVM `fstore` instruction.
    Fstore(u8),
    /// JVM `dstore` instruction.
    Dstore(u8),
    /// JVM `astore` instruction.
    Astore(u8),
    /// JVM `istore0` instruction.
    Istore0,
    /// JVM `istore1` instruction.
    Istore1,
    /// JVM `istore2` instruction.
    Istore2,
    /// JVM `istore3` instruction.
    Istore3,
    /// JVM `lstore0` instruction.
    Lstore0,
    /// JVM `lstore1` instruction.
    Lstore1,
    /// JVM `lstore2` instruction.
    Lstore2,
    /// JVM `lstore3` instruction.
    Lstore3,
    /// JVM `fstore0` instruction.
    Fstore0,
    /// JVM `fstore1` instruction.
    Fstore1,
    /// JVM `fstore2` instruction.
    Fstore2,
    /// JVM `fstore3` instruction.
    Fstore3,
    /// JVM `dstore0` instruction.
    Dstore0,
    /// JVM `dstore1` instruction.
    Dstore1,
    /// JVM `dstore2` instruction.
    Dstore2,
    /// JVM `dstore3` instruction.
    Dstore3,
    /// JVM `astore0` instruction.
    Astore0,
    /// JVM `astore1` instruction.
    Astore1,
    /// JVM `astore2` instruction.
    Astore2,
    /// JVM `astore3` instruction.
    Astore3,
    /// JVM `iastore` instruction.
    Iastore,
    /// JVM `lastore` instruction.
    Lastore,
    /// JVM `fastore` instruction.
    Fastore,
    /// JVM `dastore` instruction.
    Dastore,
    /// JVM `aastore` instruction.
    Aastore,
    /// JVM `bastore` instruction.
    Bastore,
    /// JVM `castore` instruction.
    Castore,
    /// JVM `sastore` instruction.
    Sastore,

    // -----------------------------------------------------------------------
    // Stack
    // -----------------------------------------------------------------------
    /// JVM `pop` instruction.
    Pop,
    /// JVM `pop2` instruction.
    Pop2,
    /// JVM `dup` instruction.
    Dup,
    /// JVM `dupx1` instruction.
    DupX1,
    /// JVM `dupx2` instruction.
    DupX2,
    /// JVM `dup2` instruction.
    Dup2,
    /// JVM `dup2x1` instruction.
    Dup2X1,
    /// JVM `dup2x2` instruction.
    Dup2X2,
    /// JVM `swap` instruction.
    Swap,

    // -----------------------------------------------------------------------
    // Arithmetic
    // -----------------------------------------------------------------------
    /// JVM `iadd` instruction.
    Iadd,
    /// JVM `ladd` instruction.
    Ladd,
    /// JVM `fadd` instruction.
    Fadd,
    /// JVM `dadd` instruction.
    Dadd,
    /// JVM `isub` instruction.
    Isub,
    /// JVM `lsub` instruction.
    Lsub,
    /// JVM `fsub` instruction.
    Fsub,
    /// JVM `dsub` instruction.
    Dsub,
    /// JVM `imul` instruction.
    Imul,
    /// JVM `lmul` instruction.
    Lmul,
    /// JVM `fmul` instruction.
    Fmul,
    /// JVM `dmul` instruction.
    Dmul,
    /// JVM `idiv` instruction.
    Idiv,
    /// JVM `ldiv` instruction.
    Ldiv,
    /// JVM `fdiv` instruction.
    Fdiv,
    /// JVM `ddiv` instruction.
    Ddiv,
    /// JVM `irem` instruction.
    Irem,
    /// JVM `lrem` instruction.
    Lrem,
    /// JVM `frem` instruction.
    Frem,
    /// JVM `drem` instruction.
    Drem,
    /// JVM `ineg` instruction.
    Ineg,
    /// JVM `lneg` instruction.
    Lneg,
    /// JVM `fneg` instruction.
    Fneg,
    /// JVM `dneg` instruction.
    Dneg,
    /// JVM `ishl` instruction.
    Ishl,
    /// JVM `lshl` instruction.
    Lshl,
    /// JVM `ishr` instruction.
    Ishr,
    /// JVM `lshr` instruction.
    Lshr,
    /// JVM `iushr` instruction.
    Iushr,
    /// JVM `lushr` instruction.
    Lushr,
    /// JVM `iand` instruction.
    Iand,
    /// JVM `land` instruction.
    Land,
    /// JVM `ior` instruction.
    Ior,
    /// JVM `lor` instruction.
    Lor,
    /// JVM `ixor` instruction.
    Ixor,
    /// JVM `lxor` instruction.
    Lxor,
    /// `iinc index const` — increment local by constant.
    /// JVM `iinc` instruction.
    Iinc {
        /// The `index` field.
        index: u8,
        /// The `value` field.
        value: i8,
    },

    // -----------------------------------------------------------------------
    // Conversions
    // -----------------------------------------------------------------------
    /// JVM `i2l` instruction.
    I2l,
    /// JVM `i2f` instruction.
    I2f,
    /// JVM `i2d` instruction.
    I2d,
    /// JVM `l2i` instruction.
    L2i,
    /// JVM `l2f` instruction.
    L2f,
    /// JVM `l2d` instruction.
    L2d,
    /// JVM `f2i` instruction.
    F2i,
    /// JVM `f2l` instruction.
    F2l,
    /// JVM `f2d` instruction.
    F2d,
    /// JVM `d2i` instruction.
    D2i,
    /// JVM `d2l` instruction.
    D2l,
    /// JVM `d2f` instruction.
    D2f,
    /// JVM `i2b` instruction.
    I2b,
    /// JVM `i2c` instruction.
    I2c,
    /// JVM `i2s` instruction.
    I2s,

    // -----------------------------------------------------------------------
    // Comparisons
    // -----------------------------------------------------------------------
    /// JVM `lcmp` instruction.
    Lcmp,
    /// JVM `fcmpl` instruction.
    Fcmpl,
    /// JVM `fcmpg` instruction.
    Fcmpg,
    /// JVM `dcmpl` instruction.
    Dcmpl,
    /// JVM `dcmpg` instruction.
    Dcmpg,

    // -----------------------------------------------------------------------
    // Branches — offset is relative to the PC of this instruction
    // -----------------------------------------------------------------------
    /// JVM `ifeq` instruction.
    Ifeq(i16),
    /// JVM `ifne` instruction.
    Ifne(i16),
    /// JVM `iflt` instruction.
    Iflt(i16),
    /// JVM `ifge` instruction.
    Ifge(i16),
    /// JVM `ifgt` instruction.
    Ifgt(i16),
    /// JVM `ifle` instruction.
    Ifle(i16),
    /// JVM `ificmpeq` instruction.
    IfIcmpeq(i16),
    /// JVM `ificmpne` instruction.
    IfIcmpne(i16),
    /// JVM `ificmplt` instruction.
    IfIcmplt(i16),
    /// JVM `ificmpge` instruction.
    IfIcmpge(i16),
    /// JVM `ificmpgt` instruction.
    IfIcmpgt(i16),
    /// JVM `ificmple` instruction.
    IfIcmple(i16),
    /// JVM `ifacmpeq` instruction.
    IfAcmpeq(i16),
    /// JVM `ifacmpne` instruction.
    IfAcmpne(i16),
    /// JVM `goto` instruction.
    Goto(i16),
    /// JVM `jsr` instruction.
    Jsr(i16),
    /// JVM `ret` instruction.
    Ret(u8),
    /// JVM `tableswitch` instruction.
    Tableswitch {
        /// The `default` field.
        default: i32,
        /// The `low` field.
        low: i32,
        /// The `high` field.
        high: i32,
        /// The `offsets` field.
        offsets: Vec<i32>,
    },
    /// JVM `lookupswitch` instruction.
    Lookupswitch {
        /// The `default` field.
        default: i32,
        /// The `pairs` field.
        pairs: Vec<(i32, i32)>, // (match_value, offset)
    },

    // -----------------------------------------------------------------------
    // Returns
    // -----------------------------------------------------------------------
    /// JVM `ireturn` instruction.
    Ireturn,
    /// JVM `lreturn` instruction.
    Lreturn,
    /// JVM `freturn` instruction.
    Freturn,
    /// JVM `dreturn` instruction.
    Dreturn,
    /// JVM `areturn` instruction.
    Areturn,
    /// JVM `return` instruction.
    Return,

    // -----------------------------------------------------------------------
    // Field access
    // -----------------------------------------------------------------------
    /// JVM `getstatic` instruction.
    Getstatic(CpIndex),
    /// JVM `putstatic` instruction.
    Putstatic(CpIndex),
    /// JVM `getfield` instruction.
    Getfield(CpIndex),
    /// JVM `putfield` instruction.
    Putfield(CpIndex),

    // -----------------------------------------------------------------------
    // Method invocation
    // -----------------------------------------------------------------------
    /// JVM `invokevirtual` instruction.
    Invokevirtual(CpIndex),
    /// JVM `invokespecial` instruction.
    Invokespecial(CpIndex),
    /// JVM `invokestatic` instruction.
    Invokestatic(CpIndex),
    /// `invokeinterface index count` — `count` is the number of arguments.
    /// JVM `invokeinterface` instruction.
    Invokeinterface {
        /// The `index` field.
        index: CpIndex,
        /// The `count` field.
        count: u8,
    },
    /// JVM `invokedynamic` instruction.
    Invokedynamic(CpIndex),

    // -----------------------------------------------------------------------
    // Object / array
    // -----------------------------------------------------------------------
    /// JVM `new` instruction.
    New(CpIndex),
    /// JVM `newarray` instruction.
    Newarray(ArrayType),
    /// JVM `anewarray` instruction.
    Anewarray(CpIndex),
    /// JVM `arraylength` instruction.
    Arraylength,
    /// JVM `athrow` instruction.
    Athrow,
    /// JVM `checkcast` instruction.
    Checkcast(CpIndex),
    /// JVM `instanceof` instruction.
    Instanceof(CpIndex),
    /// JVM `monitorenter` instruction.
    Monitorenter,
    /// JVM `monitorexit` instruction.
    Monitorexit,

    // -----------------------------------------------------------------------
    // Extended
    // -----------------------------------------------------------------------
    /// JVM `multianewarray` instruction.
    Multianewarray {
        /// The `index` field.
        index: CpIndex,
        /// The `dimensions` field.
        dimensions: u8,
    },
    /// JVM `ifnull` instruction.
    Ifnull(i16),
    /// JVM `ifnonnull` instruction.
    Ifnonnull(i16),
    /// JVM `gotow` instruction.
    GotoW(i32),
    /// JVM `jsrw` instruction.
    JsrW(i32),

    // -----------------------------------------------------------------------
    // Wide-prefixed variants (u16 index instead of u8)
    // -----------------------------------------------------------------------
    /// JVM `iloadw` instruction.
    IloadW(u16),
    /// JVM `lloadw` instruction.
    LloadW(u16),
    /// JVM `floadw` instruction.
    FloadW(u16),
    /// JVM `dloadw` instruction.
    DloadW(u16),
    /// JVM `aloadw` instruction.
    AloadW(u16),
    /// JVM `istorew` instruction.
    IstoreW(u16),
    /// JVM `lstorew` instruction.
    LstoreW(u16),
    /// JVM `fstorew` instruction.
    FstoreW(u16),
    /// JVM `dstorew` instruction.
    DstoreW(u16),
    /// JVM `astorew` instruction.
    AstoreW(u16),
    /// JVM `retw` instruction.
    RetW(u16),
    /// JVM `iincw` instruction.
    IincW {
        /// The `index` field.
        index: u16,
        /// The `value` field.
        value: i16,
    },
}

/// An iterator over the match values and offsets of a switch statement.
///
/// This unifies the `tableswitch` and `lookupswitch` instructions into a common iteration pattern.
/// The JVM executes a switch by evaluating a key and jumping to the corresponding offset.
/// If no key matches, execution jumps to the `default` offset.
///
/// # Examples
///
/// ```
/// use duke_bytecode::Instruction;
///
/// let switch = Instruction::Lookupswitch {
///     default: 10,
///     pairs: vec![(5, 42), (7, 50)],
/// };
///
/// let (default_target, mut targets) = switch.switch_targets().unwrap();
/// assert_eq!(default_target, 10);
/// assert_eq!(targets.next(), Some((5, 42)));
/// assert_eq!(targets.next(), Some((7, 50)));
/// assert_eq!(targets.next(), None);
/// ```
pub enum SwitchTargets<'a> {
    /// Iterator for `tableswitch` instruction.
    Tableswitch {
        /// The lowest match value.
        low: i32,
        /// Iterator over the offsets.
        offsets: std::slice::Iter<'a, i32>,
        /// The current match value.
        current: i32,
    },
    /// Iterator for `lookupswitch` instruction.
    Lookupswitch(std::slice::Iter<'a, (i32, i32)>),
}

impl Iterator for SwitchTargets<'_> {
    type Item = (i32, i32);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Tableswitch {
                low: _,
                offsets,
                current,
            } => {
                let offset = offsets.next()?;
                let val = *current;
                *current = current.wrapping_add(1);
                Some((val, *offset))
            }
            Self::Lookupswitch(pairs) => pairs.next().copied(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Tableswitch { offsets, .. } => offsets.size_hint(),
            Self::Lookupswitch(pairs) => pairs.size_hint(),
        }
    }
}

impl ExactSizeIterator for SwitchTargets<'_> {}

impl Instruction {
    /// Returns `true` if the instruction is a conditional branch.
    #[must_use]
    pub const fn is_conditional_branch(&self) -> bool {
        matches!(
            self,
            Self::Ifeq(_)
                | Self::Ifne(_)
                | Self::Iflt(_)
                | Self::Ifge(_)
                | Self::Ifgt(_)
                | Self::Ifle(_)
                | Self::IfIcmpeq(_)
                | Self::IfIcmpne(_)
                | Self::IfIcmplt(_)
                | Self::IfIcmpge(_)
                | Self::IfIcmpgt(_)
                | Self::IfIcmple(_)
                | Self::IfAcmpeq(_)
                | Self::IfAcmpne(_)
                | Self::Ifnull(_)
                | Self::Ifnonnull(_)
        )
    }

    /// Returns the branch target offset for a conditional branch.
    #[must_use]
    pub const fn conditional_branch_target(&self) -> Option<isize> {
        match self {
            Self::Ifeq(offset)
            | Self::Ifne(offset)
            | Self::Iflt(offset)
            | Self::Ifge(offset)
            | Self::Ifgt(offset)
            | Self::Ifle(offset)
            | Self::IfIcmpeq(offset)
            | Self::IfIcmpne(offset)
            | Self::IfIcmplt(offset)
            | Self::IfIcmpge(offset)
            | Self::IfIcmpgt(offset)
            | Self::IfIcmple(offset)
            | Self::IfAcmpeq(offset)
            | Self::IfAcmpne(offset)
            | Self::Ifnull(offset)
            | Self::Ifnonnull(offset) => Some(*offset as isize),
            _ => None,
        }
    }

    /// Returns `true` if the instruction is an unconditional jump.
    #[must_use]
    pub const fn is_unconditional_jump(&self) -> bool {
        matches!(
            self,
            Self::Goto(_) | Self::GotoW(_) | Self::Jsr(_) | Self::JsrW(_)
        )
    }

    /// Returns `true` if the instruction is a subroutine call.
    #[must_use]
    pub const fn is_subroutine_call(&self) -> bool {
        matches!(self, Self::Jsr(_) | Self::JsrW(_))
    }

    /// Returns the target offset for an unconditional jump.
    #[must_use]
    pub const fn unconditional_jump_target(&self) -> Option<isize> {
        match self {
            Self::Goto(offset) | Self::Jsr(offset) => Some(*offset as isize),
            Self::GotoW(offset) | Self::JsrW(offset) => Some(*offset as isize),
            _ => None,
        }
    }

    /// Returns the switch targets if the instruction is a switch statement.
    /// It returns `Some((default_offset, targets_iterator))`.
    ///
    /// ⚡ Bolt: By returning an iterator instead of allocating and collecting into
    /// a new `Vec`, we eliminate heap allocations during CFG generation and
    /// complexity calculation.
    #[must_use]
    pub fn switch_targets(&self) -> Option<(i32, SwitchTargets<'_>)> {
        match self {
            Self::Tableswitch {
                default,
                low,
                offsets,
                ..
            } => Some((
                *default,
                SwitchTargets::Tableswitch {
                    low: *low,
                    offsets: offsets.iter(),
                    current: *low,
                },
            )),
            Self::Lookupswitch { default, pairs } => {
                Some((*default, SwitchTargets::Lookupswitch(pairs.iter())))
            }
            _ => None,
        }
    }

    /// Returns `true` if the instruction is a switch statement.
    #[must_use]
    pub const fn is_switch(&self) -> bool {
        matches!(self, Self::Tableswitch { .. } | Self::Lookupswitch { .. })
    }

    /// Returns `true` if the instruction halts execution in the current frame.
    #[must_use]
    pub const fn is_return(&self) -> bool {
        matches!(
            self,
            Self::Return
                | Self::Ireturn
                | Self::Lreturn
                | Self::Freturn
                | Self::Dreturn
                | Self::Areturn
                | Self::Athrow
                | Self::Ret(_)
                | Self::RetW(_)
        )
    }

    /// Returns all possible control flow targets (next PC values) from this instruction,
    /// including fall-through if applicable.
    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        clippy::collapsible_if,
        clippy::collapsible_else_if,
        clippy::needless_else
    )]
    #[must_use]
    /// Optimization: Pre-allocate capacity of 2 since branch targets and fallthroughs max out at two.
    /// Reduces dynamic reallocation overhead during CFG construction.
    pub fn control_flow_targets(&self, current_pc: usize, next_pc: Option<usize>) -> Vec<usize> {
        if self.is_return() {
            return Vec::new();
        }
        let mut targets = Vec::with_capacity(2);
        if let Some(offset) = self.unconditional_jump_target() {
            targets.push((current_pc as isize + offset) as usize);
            if self.is_subroutine_call() {
                if let Some(next) = next_pc {
                    targets.push(next);
                }
            }
        } else if let Some(offset) = self.conditional_branch_target() {
            targets.push((current_pc as isize + offset) as usize);
            if let Some(next) = next_pc {
                targets.push(next);
            }
        } else if let Some((default, pairs)) = self.switch_targets() {
            targets.push((current_pc as isize + default as isize) as usize);
            for (_, offset) in pairs {
                targets.push((current_pc as isize + offset as isize) as usize);
            }
        } else {
            if let Some(next) = next_pc {
                targets.push(next);
            }
        }
        targets
    }

    /// Returns a list of target PCs and their optional edge labels (e.g., `"true"`, `"false"`, `"default"`)
    /// for control flow graph generation.
    #[must_use]
    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        clippy::collapsible_if,
        clippy::collapsible_else_if
    )]
    /// Optimization: Pre-allocate capacity of 2 since branch targets and fallthroughs max out at two.
    /// Reduces dynamic reallocation overhead during CFG construction.
    pub fn control_flow_edges(
        &self,
        current_pc: usize,
        next_pc: Option<usize>,
    ) -> Vec<(usize, Option<String>)> {
        if self.is_return() {
            return Vec::new();
        }
        let mut edges = Vec::with_capacity(2);
        if let Some(offset) = self.unconditional_jump_target() {
            edges.push(((current_pc as isize + offset) as usize, None));
            if self.is_subroutine_call() {
                if let Some(next) = next_pc {
                    edges.push((next, Some("false".to_string())));
                    edges[0].1 = Some("true".to_string());
                }
            }
        } else if let Some(offset) = self.conditional_branch_target() {
            edges.push((
                (current_pc as isize + offset) as usize,
                Some("true".to_string()),
            ));
            if let Some(next) = next_pc {
                edges.push((next, Some("false".to_string())));
            }
        } else if let Some((default, pairs)) = self.switch_targets() {
            edges.push((
                (current_pc as isize + default as isize) as usize,
                Some("default".to_string()),
            ));
            for (val, offset) in pairs {
                edges.push((
                    (current_pc as isize + offset as isize) as usize,
                    Some(val.to_string()),
                ));
            }
        } else {
            if let Some(next) = next_pc {
                edges.push((next, None));
            }
        }
        edges
    }

    /// Returns the mnemonic string for display/debugging.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub const fn mnemonic(&self) -> &'static str {
        match self {
            Self::Nop => "nop",
            Self::AconstNull => "aconst_null",
            Self::IconstM1 => "iconst_m1",
            Self::Iconst0 => "iconst_0",
            Self::Iconst1 => "iconst_1",
            Self::Iconst2 => "iconst_2",
            Self::Iconst3 => "iconst_3",
            Self::Iconst4 => "iconst_4",
            Self::Iconst5 => "iconst_5",
            Self::Lconst0 => "lconst_0",
            Self::Lconst1 => "lconst_1",
            Self::Fconst0 => "fconst_0",
            Self::Fconst1 => "fconst_1",
            Self::Fconst2 => "fconst_2",
            Self::Dconst0 => "dconst_0",
            Self::Dconst1 => "dconst_1",
            Self::Bipush(_) => "bipush",
            Self::Sipush(_) => "sipush",
            Self::Ldc(_) => "ldc",
            Self::LdcW(_) => "ldc_w",
            Self::Ldc2W(_) => "ldc2_w",
            Self::Iload(_) => "iload",
            Self::Lload(_) => "lload",
            Self::Fload(_) => "fload",
            Self::Dload(_) => "dload",
            Self::Aload(_) => "aload",
            Self::Iload0 => "iload_0",
            Self::Iload1 => "iload_1",
            Self::Iload2 => "iload_2",
            Self::Iload3 => "iload_3",
            Self::Lload0 => "lload_0",
            Self::Lload1 => "lload_1",
            Self::Lload2 => "lload_2",
            Self::Lload3 => "lload_3",
            Self::Fload0 => "fload_0",
            Self::Fload1 => "fload_1",
            Self::Fload2 => "fload_2",
            Self::Fload3 => "fload_3",
            Self::Dload0 => "dload_0",
            Self::Dload1 => "dload_1",
            Self::Dload2 => "dload_2",
            Self::Dload3 => "dload_3",
            Self::Aload0 => "aload_0",
            Self::Aload1 => "aload_1",
            Self::Aload2 => "aload_2",
            Self::Aload3 => "aload_3",
            Self::Iaload => "iaload",
            Self::Laload => "laload",
            Self::Faload => "faload",
            Self::Daload => "daload",
            Self::Aaload => "aaload",
            Self::Baload => "baload",
            Self::Caload => "caload",
            Self::Saload => "saload",
            Self::Istore(_) => "istore",
            Self::Lstore(_) => "lstore",
            Self::Fstore(_) => "fstore",
            Self::Dstore(_) => "dstore",
            Self::Astore(_) => "astore",
            Self::Istore0 => "istore_0",
            Self::Istore1 => "istore_1",
            Self::Istore2 => "istore_2",
            Self::Istore3 => "istore_3",
            Self::Lstore0 => "lstore_0",
            Self::Lstore1 => "lstore_1",
            Self::Lstore2 => "lstore_2",
            Self::Lstore3 => "lstore_3",
            Self::Fstore0 => "fstore_0",
            Self::Fstore1 => "fstore_1",
            Self::Fstore2 => "fstore_2",
            Self::Fstore3 => "fstore_3",
            Self::Dstore0 => "dstore_0",
            Self::Dstore1 => "dstore_1",
            Self::Dstore2 => "dstore_2",
            Self::Dstore3 => "dstore_3",
            Self::Astore0 => "astore_0",
            Self::Astore1 => "astore_1",
            Self::Astore2 => "astore_2",
            Self::Astore3 => "astore_3",
            Self::Iastore => "iastore",
            Self::Lastore => "lastore",
            Self::Fastore => "fastore",
            Self::Dastore => "dastore",
            Self::Aastore => "aastore",
            Self::Bastore => "bastore",
            Self::Castore => "castore",
            Self::Sastore => "sastore",
            Self::Pop => "pop",
            Self::Pop2 => "pop2",
            Self::Dup => "dup",
            Self::DupX1 => "dup_x1",
            Self::DupX2 => "dup_x2",
            Self::Dup2 => "dup2",
            Self::Dup2X1 => "dup2_x1",
            Self::Dup2X2 => "dup2_x2",
            Self::Swap => "swap",
            Self::Iadd => "iadd",
            Self::Ladd => "ladd",
            Self::Fadd => "fadd",
            Self::Dadd => "dadd",
            Self::Isub => "isub",
            Self::Lsub => "lsub",
            Self::Fsub => "fsub",
            Self::Dsub => "dsub",
            Self::Imul => "imul",
            Self::Lmul => "lmul",
            Self::Fmul => "fmul",
            Self::Dmul => "dmul",
            Self::Idiv => "idiv",
            Self::Ldiv => "ldiv",
            Self::Fdiv => "fdiv",
            Self::Ddiv => "ddiv",
            Self::Irem => "irem",
            Self::Lrem => "lrem",
            Self::Frem => "frem",
            Self::Drem => "drem",
            Self::Ineg => "ineg",
            Self::Lneg => "lneg",
            Self::Fneg => "fneg",
            Self::Dneg => "dneg",
            Self::Ishl => "ishl",
            Self::Lshl => "lshl",
            Self::Ishr => "ishr",
            Self::Lshr => "lshr",
            Self::Iushr => "iushr",
            Self::Lushr => "lushr",
            Self::Iand => "iand",
            Self::Land => "land",
            Self::Ior => "ior",
            Self::Lor => "lor",
            Self::Ixor => "ixor",
            Self::Lxor => "lxor",
            Self::Iinc { .. } => "iinc",
            Self::I2l => "i2l",
            Self::I2f => "i2f",
            Self::I2d => "i2d",
            Self::L2i => "l2i",
            Self::L2f => "l2f",
            Self::L2d => "l2d",
            Self::F2i => "f2i",
            Self::F2l => "f2l",
            Self::F2d => "f2d",
            Self::D2i => "d2i",
            Self::D2l => "d2l",
            Self::D2f => "d2f",
            Self::I2b => "i2b",
            Self::I2c => "i2c",
            Self::I2s => "i2s",
            Self::Lcmp => "lcmp",
            Self::Fcmpl => "fcmpl",
            Self::Fcmpg => "fcmpg",
            Self::Dcmpl => "dcmpl",
            Self::Dcmpg => "dcmpg",
            Self::Ifeq(_) => "ifeq",
            Self::Ifne(_) => "ifne",
            Self::Iflt(_) => "iflt",
            Self::Ifge(_) => "ifge",
            Self::Ifgt(_) => "ifgt",
            Self::Ifle(_) => "ifle",
            Self::IfIcmpeq(_) => "if_icmpeq",
            Self::IfIcmpne(_) => "if_icmpne",
            Self::IfIcmplt(_) => "if_icmplt",
            Self::IfIcmpge(_) => "if_icmpge",
            Self::IfIcmpgt(_) => "if_icmpgt",
            Self::IfIcmple(_) => "if_icmple",
            Self::IfAcmpeq(_) => "if_acmpeq",
            Self::IfAcmpne(_) => "if_acmpne",
            Self::Goto(_) => "goto",
            Self::Jsr(_) => "jsr",
            Self::Ret(_) => "ret",
            Self::Tableswitch { .. } => "tableswitch",
            Self::Lookupswitch { .. } => "lookupswitch",
            Self::Ireturn => "ireturn",
            Self::Lreturn => "lreturn",
            Self::Freturn => "freturn",
            Self::Dreturn => "dreturn",
            Self::Areturn => "areturn",
            Self::Return => "return",
            Self::Getstatic(_) => "getstatic",
            Self::Putstatic(_) => "putstatic",
            Self::Getfield(_) => "getfield",
            Self::Putfield(_) => "putfield",
            Self::Invokevirtual(_) => "invokevirtual",
            Self::Invokespecial(_) => "invokespecial",
            Self::Invokestatic(_) => "invokestatic",
            Self::Invokeinterface { .. } => "invokeinterface",
            Self::Invokedynamic(_) => "invokedynamic",
            Self::New(_) => "new",
            Self::Newarray(_) => "newarray",
            Self::Anewarray(_) => "anewarray",
            Self::Arraylength => "arraylength",
            Self::Athrow => "athrow",
            Self::Checkcast(_) => "checkcast",
            Self::Instanceof(_) => "instanceof",
            Self::Monitorenter => "monitorenter",
            Self::Monitorexit => "monitorexit",
            Self::Multianewarray { .. } => "multianewarray",
            Self::Ifnull(_) => "ifnull",
            Self::Ifnonnull(_) => "ifnonnull",
            Self::GotoW(_) => "goto_w",
            Self::JsrW(_) => "jsr_w",
            Self::IloadW(_) => "wide iload",
            Self::LloadW(_) => "wide lload",
            Self::FloadW(_) => "wide fload",
            Self::DloadW(_) => "wide dload",
            Self::AloadW(_) => "wide aload",
            Self::IstoreW(_) => "wide istore",
            Self::LstoreW(_) => "wide lstore",
            Self::FstoreW(_) => "wide fstore",
            Self::DstoreW(_) => "wide dstore",
            Self::AstoreW(_) => "wide astore",
            Self::RetW(_) => "wide ret",
            Self::IincW { .. } => "wide iinc",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_type_from_u8() {
        assert_eq!(ArrayType::from_u8(4), Some(ArrayType::Boolean));
        assert_eq!(ArrayType::from_u8(5), Some(ArrayType::Char));
        assert_eq!(ArrayType::from_u8(6), Some(ArrayType::Float));
        assert_eq!(ArrayType::from_u8(7), Some(ArrayType::Double));
        assert_eq!(ArrayType::from_u8(8), Some(ArrayType::Byte));
        assert_eq!(ArrayType::from_u8(9), Some(ArrayType::Short));
        assert_eq!(ArrayType::from_u8(10), Some(ArrayType::Int));
        assert_eq!(ArrayType::from_u8(11), Some(ArrayType::Long));
        assert_eq!(ArrayType::from_u8(0), None);
        assert_eq!(ArrayType::from_u8(12), None);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_instruction_mnemonics() {
        let cases = vec![
            (Instruction::Nop, "nop"),
            (Instruction::AconstNull, "aconst_null"),
            (Instruction::IconstM1, "iconst_m1"),
            (Instruction::Iconst0, "iconst_0"),
            (Instruction::Iconst1, "iconst_1"),
            (Instruction::Iconst2, "iconst_2"),
            (Instruction::Iconst3, "iconst_3"),
            (Instruction::Iconst4, "iconst_4"),
            (Instruction::Iconst5, "iconst_5"),
            (Instruction::Lconst0, "lconst_0"),
            (Instruction::Lconst1, "lconst_1"),
            (Instruction::Fconst0, "fconst_0"),
            (Instruction::Fconst1, "fconst_1"),
            (Instruction::Fconst2, "fconst_2"),
            (Instruction::Dconst0, "dconst_0"),
            (Instruction::Dconst1, "dconst_1"),
            (Instruction::Bipush(0), "bipush"),
            (Instruction::Sipush(0), "sipush"),
            (Instruction::Ldc(0), "ldc"),
            (Instruction::LdcW(CpIndex(0)), "ldc_w"),
            (Instruction::Ldc2W(CpIndex(0)), "ldc2_w"),
            (Instruction::Iload(0), "iload"),
            (Instruction::Lload(0), "lload"),
            (Instruction::Fload(0), "fload"),
            (Instruction::Dload(0), "dload"),
            (Instruction::Aload(0), "aload"),
            (Instruction::Iload0, "iload_0"),
            (Instruction::Iload1, "iload_1"),
            (Instruction::Iload2, "iload_2"),
            (Instruction::Iload3, "iload_3"),
            (Instruction::Lload0, "lload_0"),
            (Instruction::Lload1, "lload_1"),
            (Instruction::Lload2, "lload_2"),
            (Instruction::Lload3, "lload_3"),
            (Instruction::Fload0, "fload_0"),
            (Instruction::Fload1, "fload_1"),
            (Instruction::Fload2, "fload_2"),
            (Instruction::Fload3, "fload_3"),
            (Instruction::Dload0, "dload_0"),
            (Instruction::Dload1, "dload_1"),
            (Instruction::Dload2, "dload_2"),
            (Instruction::Dload3, "dload_3"),
            (Instruction::Aload0, "aload_0"),
            (Instruction::Aload1, "aload_1"),
            (Instruction::Aload2, "aload_2"),
            (Instruction::Aload3, "aload_3"),
            (Instruction::Iaload, "iaload"),
            (Instruction::Laload, "laload"),
            (Instruction::Faload, "faload"),
            (Instruction::Daload, "daload"),
            (Instruction::Aaload, "aaload"),
            (Instruction::Baload, "baload"),
            (Instruction::Caload, "caload"),
            (Instruction::Saload, "saload"),
            (Instruction::Istore(0), "istore"),
            (Instruction::Lstore(0), "lstore"),
            (Instruction::Fstore(0), "fstore"),
            (Instruction::Dstore(0), "dstore"),
            (Instruction::Astore(0), "astore"),
            (Instruction::Istore0, "istore_0"),
            (Instruction::Istore1, "istore_1"),
            (Instruction::Istore2, "istore_2"),
            (Instruction::Istore3, "istore_3"),
            (Instruction::Lstore0, "lstore_0"),
            (Instruction::Lstore1, "lstore_1"),
            (Instruction::Lstore2, "lstore_2"),
            (Instruction::Lstore3, "lstore_3"),
            (Instruction::Fstore0, "fstore_0"),
            (Instruction::Fstore1, "fstore_1"),
            (Instruction::Fstore2, "fstore_2"),
            (Instruction::Fstore3, "fstore_3"),
            (Instruction::Dstore0, "dstore_0"),
            (Instruction::Dstore1, "dstore_1"),
            (Instruction::Dstore2, "dstore_2"),
            (Instruction::Dstore3, "dstore_3"),
            (Instruction::Astore0, "astore_0"),
            (Instruction::Astore1, "astore_1"),
            (Instruction::Astore2, "astore_2"),
            (Instruction::Astore3, "astore_3"),
            (Instruction::Iastore, "iastore"),
            (Instruction::Lastore, "lastore"),
            (Instruction::Fastore, "fastore"),
            (Instruction::Dastore, "dastore"),
            (Instruction::Aastore, "aastore"),
            (Instruction::Bastore, "bastore"),
            (Instruction::Castore, "castore"),
            (Instruction::Sastore, "sastore"),
            (Instruction::Pop, "pop"),
            (Instruction::Pop2, "pop2"),
            (Instruction::Dup, "dup"),
            (Instruction::DupX1, "dup_x1"),
            (Instruction::DupX2, "dup_x2"),
            (Instruction::Dup2, "dup2"),
            (Instruction::Dup2X1, "dup2_x1"),
            (Instruction::Dup2X2, "dup2_x2"),
            (Instruction::Swap, "swap"),
            (Instruction::Iadd, "iadd"),
            (Instruction::Ladd, "ladd"),
            (Instruction::Fadd, "fadd"),
            (Instruction::Dadd, "dadd"),
            (Instruction::Isub, "isub"),
            (Instruction::Lsub, "lsub"),
            (Instruction::Fsub, "fsub"),
            (Instruction::Dsub, "dsub"),
            (Instruction::Imul, "imul"),
            (Instruction::Lmul, "lmul"),
            (Instruction::Fmul, "fmul"),
            (Instruction::Dmul, "dmul"),
            (Instruction::Idiv, "idiv"),
            (Instruction::Ldiv, "ldiv"),
            (Instruction::Fdiv, "fdiv"),
            (Instruction::Ddiv, "ddiv"),
            (Instruction::Irem, "irem"),
            (Instruction::Lrem, "lrem"),
            (Instruction::Frem, "frem"),
            (Instruction::Drem, "drem"),
            (Instruction::Ineg, "ineg"),
            (Instruction::Lneg, "lneg"),
            (Instruction::Fneg, "fneg"),
            (Instruction::Dneg, "dneg"),
            (Instruction::Ishl, "ishl"),
            (Instruction::Lshl, "lshl"),
            (Instruction::Ishr, "ishr"),
            (Instruction::Lshr, "lshr"),
            (Instruction::Iushr, "iushr"),
            (Instruction::Lushr, "lushr"),
            (Instruction::Iand, "iand"),
            (Instruction::Land, "land"),
            (Instruction::Ior, "ior"),
            (Instruction::Lor, "lor"),
            (Instruction::Ixor, "ixor"),
            (Instruction::Lxor, "lxor"),
            (Instruction::Iinc { index: 0, value: 0 }, "iinc"),
            (Instruction::I2l, "i2l"),
            (Instruction::I2f, "i2f"),
            (Instruction::I2d, "i2d"),
            (Instruction::L2i, "l2i"),
            (Instruction::L2f, "l2f"),
            (Instruction::L2d, "l2d"),
            (Instruction::F2i, "f2i"),
            (Instruction::F2l, "f2l"),
            (Instruction::F2d, "f2d"),
            (Instruction::D2i, "d2i"),
            (Instruction::D2l, "d2l"),
            (Instruction::D2f, "d2f"),
            (Instruction::I2b, "i2b"),
            (Instruction::I2c, "i2c"),
            (Instruction::I2s, "i2s"),
            (Instruction::Lcmp, "lcmp"),
            (Instruction::Fcmpl, "fcmpl"),
            (Instruction::Fcmpg, "fcmpg"),
            (Instruction::Dcmpl, "dcmpl"),
            (Instruction::Dcmpg, "dcmpg"),
            (Instruction::Ifeq(0), "ifeq"),
            (Instruction::Ifne(0), "ifne"),
            (Instruction::Iflt(0), "iflt"),
            (Instruction::Ifge(0), "ifge"),
            (Instruction::Ifgt(0), "ifgt"),
            (Instruction::Ifle(0), "ifle"),
            (Instruction::IfIcmpeq(0), "if_icmpeq"),
            (Instruction::IfIcmpne(0), "if_icmpne"),
            (Instruction::IfIcmplt(0), "if_icmplt"),
            (Instruction::IfIcmpge(0), "if_icmpge"),
            (Instruction::IfIcmpgt(0), "if_icmpgt"),
            (Instruction::IfIcmple(0), "if_icmple"),
            (Instruction::IfAcmpeq(0), "if_acmpeq"),
            (Instruction::IfAcmpne(0), "if_acmpne"),
            (Instruction::Goto(0), "goto"),
            (Instruction::Jsr(0), "jsr"),
            (Instruction::Ret(0), "ret"),
            (
                Instruction::Tableswitch {
                    default: 0,
                    low: 0,
                    high: 0,
                    offsets: vec![],
                },
                "tableswitch",
            ),
            (
                Instruction::Lookupswitch {
                    default: 0,
                    pairs: vec![],
                },
                "lookupswitch",
            ),
            (Instruction::Ireturn, "ireturn"),
            (Instruction::Lreturn, "lreturn"),
            (Instruction::Freturn, "freturn"),
            (Instruction::Dreturn, "dreturn"),
            (Instruction::Areturn, "areturn"),
            (Instruction::Return, "return"),
            (Instruction::Getstatic(CpIndex(0)), "getstatic"),
            (Instruction::Putstatic(CpIndex(0)), "putstatic"),
            (Instruction::Getfield(CpIndex(0)), "getfield"),
            (Instruction::Putfield(CpIndex(0)), "putfield"),
            (Instruction::Invokevirtual(CpIndex(0)), "invokevirtual"),
            (Instruction::Invokespecial(CpIndex(0)), "invokespecial"),
            (Instruction::Invokestatic(CpIndex(0)), "invokestatic"),
            (
                Instruction::Invokeinterface {
                    index: CpIndex(0),
                    count: 0,
                },
                "invokeinterface",
            ),
            (Instruction::Invokedynamic(CpIndex(0)), "invokedynamic"),
            (Instruction::New(CpIndex(0)), "new"),
            (Instruction::Newarray(ArrayType::Boolean), "newarray"),
            (Instruction::Anewarray(CpIndex(0)), "anewarray"),
            (Instruction::Arraylength, "arraylength"),
            (Instruction::Athrow, "athrow"),
            (Instruction::Checkcast(CpIndex(0)), "checkcast"),
            (Instruction::Instanceof(CpIndex(0)), "instanceof"),
            (Instruction::Monitorenter, "monitorenter"),
            (Instruction::Monitorexit, "monitorexit"),
            (
                Instruction::Multianewarray {
                    index: CpIndex(0),
                    dimensions: 0,
                },
                "multianewarray",
            ),
            (Instruction::Ifnull(0), "ifnull"),
            (Instruction::Ifnonnull(0), "ifnonnull"),
            (Instruction::GotoW(0), "goto_w"),
            (Instruction::JsrW(0), "jsr_w"),
            (Instruction::IloadW(0), "wide iload"),
            (Instruction::LloadW(0), "wide lload"),
            (Instruction::FloadW(0), "wide fload"),
            (Instruction::DloadW(0), "wide dload"),
            (Instruction::AloadW(0), "wide aload"),
            (Instruction::IstoreW(0), "wide istore"),
            (Instruction::LstoreW(0), "wide lstore"),
            (Instruction::FstoreW(0), "wide fstore"),
            (Instruction::DstoreW(0), "wide dstore"),
            (Instruction::AstoreW(0), "wide astore"),
            (Instruction::RetW(0), "wide ret"),
            (Instruction::IincW { index: 0, value: 0 }, "wide iinc"),
        ];

        for (instr, expected) in cases {
            assert_eq!(instr.mnemonic(), expected);
        }
    }

    #[test]
    fn test_is_conditional_branch() {
        assert!(Instruction::Ifeq(5).is_conditional_branch());
        assert!(Instruction::Ifne(5).is_conditional_branch());
        assert!(!Instruction::Iconst0.is_conditional_branch());
    }

    #[test]
    fn test_conditional_branch_target() {
        assert_eq!(Instruction::Ifeq(5).conditional_branch_target(), Some(5));
        assert_eq!(Instruction::Ifne(5).conditional_branch_target(), Some(5));
        assert_eq!(Instruction::Iconst0.conditional_branch_target(), None);
    }

    #[test]
    fn test_is_unconditional_jump() {
        assert!(Instruction::Goto(5).is_unconditional_jump());
        assert!(Instruction::GotoW(5).is_unconditional_jump());
        assert!(Instruction::Jsr(5).is_unconditional_jump());
        assert!(Instruction::JsrW(5).is_unconditional_jump());
        assert!(!Instruction::Iconst0.is_unconditional_jump());
    }

    #[test]
    fn test_unconditional_jump_target() {
        assert_eq!(Instruction::Goto(5).unconditional_jump_target(), Some(5));
        assert_eq!(Instruction::GotoW(5).unconditional_jump_target(), Some(5));
        assert_eq!(Instruction::Jsr(5).unconditional_jump_target(), Some(5));
        assert_eq!(Instruction::JsrW(5).unconditional_jump_target(), Some(5));
        assert_eq!(Instruction::Iconst0.unconditional_jump_target(), None);
    }

    #[test]
    fn test_is_subroutine_call() {
        assert!(Instruction::Jsr(5).is_subroutine_call());
        assert!(Instruction::JsrW(5).is_subroutine_call());
        assert!(!Instruction::Goto(5).is_subroutine_call());
        assert!(!Instruction::Iconst0.is_subroutine_call());
    }

    #[test]
    fn test_switch_targets() {
        let tableswitch = Instruction::Tableswitch {
            default: 5,
            low: 10,
            high: 12,
            offsets: vec![100, 200, 300],
        };
        let targets = tableswitch.switch_targets();
        assert!(targets.is_some());
        let (default, iter) = targets.unwrap();
        assert_eq!(default, 5);
        assert_eq!(
            iter.collect::<Vec<_>>(),
            vec![(10, 100), (11, 200), (12, 300)]
        );

        let lookupswitch = Instruction::Lookupswitch {
            default: 5,
            pairs: vec![(10, 100), (20, 200)],
        };
        let targets = lookupswitch.switch_targets();
        assert!(targets.is_some());
        let (default, iter) = targets.unwrap();
        assert_eq!(default, 5);
        assert_eq!(iter.collect::<Vec<_>>(), vec![(10, 100), (20, 200)]);

        assert!(Instruction::Iconst0.switch_targets().is_none());
    }

    #[test]
    fn test_is_switch() {
        assert!(
            Instruction::Tableswitch {
                default: 0,
                low: 0,
                high: 0,
                offsets: vec![],
            }
            .is_switch()
        );
        assert!(
            Instruction::Lookupswitch {
                default: 0,
                pairs: vec![],
            }
            .is_switch()
        );
        assert!(!Instruction::Iconst0.is_switch());
    }

    #[test]
    fn test_is_return() {
        assert!(Instruction::Return.is_return());
        assert!(Instruction::Ireturn.is_return());
        assert!(Instruction::Athrow.is_return());
        assert!(!Instruction::Iconst0.is_return());
    }

    #[test]
    fn test_instruction_is_conditional_branch() {
        assert!(Instruction::Ifeq(0).is_conditional_branch());
        assert!(Instruction::Ifne(0).is_conditional_branch());
        assert!(Instruction::Iflt(0).is_conditional_branch());
        assert!(Instruction::Ifge(0).is_conditional_branch());
        assert!(Instruction::Ifgt(0).is_conditional_branch());
        assert!(Instruction::Ifle(0).is_conditional_branch());
        assert!(Instruction::IfIcmpeq(0).is_conditional_branch());
        assert!(Instruction::IfIcmpne(0).is_conditional_branch());
        assert!(Instruction::IfIcmplt(0).is_conditional_branch());
        assert!(Instruction::IfIcmpge(0).is_conditional_branch());
        assert!(Instruction::IfIcmpgt(0).is_conditional_branch());
        assert!(Instruction::IfIcmple(0).is_conditional_branch());
        assert!(Instruction::IfAcmpeq(0).is_conditional_branch());
        assert!(Instruction::IfAcmpne(0).is_conditional_branch());
        assert!(Instruction::Ifnull(0).is_conditional_branch());
        assert!(Instruction::Ifnonnull(0).is_conditional_branch());

        assert!(!Instruction::Goto(0).is_conditional_branch());
        assert!(!Instruction::Nop.is_conditional_branch());
    }

    #[test]
    fn test_instruction_conditional_branch_target() {
        assert_eq!(Instruction::Ifeq(10).conditional_branch_target(), Some(10));
        assert_eq!(Instruction::Ifne(-5).conditional_branch_target(), Some(-5));
        assert_eq!(Instruction::Iflt(0).conditional_branch_target(), Some(0));
        assert_eq!(Instruction::Ifge(10).conditional_branch_target(), Some(10));
        assert_eq!(Instruction::Ifgt(10).conditional_branch_target(), Some(10));
        assert_eq!(Instruction::Ifle(10).conditional_branch_target(), Some(10));
        assert_eq!(
            Instruction::IfIcmpeq(10).conditional_branch_target(),
            Some(10)
        );
        assert_eq!(
            Instruction::IfIcmpne(10).conditional_branch_target(),
            Some(10)
        );
        assert_eq!(
            Instruction::IfIcmplt(10).conditional_branch_target(),
            Some(10)
        );
        assert_eq!(
            Instruction::IfIcmpge(10).conditional_branch_target(),
            Some(10)
        );
        assert_eq!(
            Instruction::IfIcmpgt(10).conditional_branch_target(),
            Some(10)
        );
        assert_eq!(
            Instruction::IfIcmple(10).conditional_branch_target(),
            Some(10)
        );
        assert_eq!(
            Instruction::IfAcmpeq(10).conditional_branch_target(),
            Some(10)
        );
        assert_eq!(
            Instruction::IfAcmpne(10).conditional_branch_target(),
            Some(10)
        );
        assert_eq!(
            Instruction::Ifnull(10).conditional_branch_target(),
            Some(10)
        );
        assert_eq!(
            Instruction::Ifnonnull(10).conditional_branch_target(),
            Some(10)
        );

        assert_eq!(Instruction::Goto(10).conditional_branch_target(), None);
        assert_eq!(Instruction::Nop.conditional_branch_target(), None);
    }

    #[test]
    fn test_instruction_is_unconditional_jump() {
        assert!(Instruction::Goto(0).is_unconditional_jump());
        assert!(Instruction::GotoW(0).is_unconditional_jump());

        assert!(!Instruction::Ifeq(0).is_unconditional_jump());
        assert!(!Instruction::Nop.is_unconditional_jump());
    }
}
