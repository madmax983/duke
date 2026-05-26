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
    /// Do nothing.
    Nop,
    /// Push `null`.
    AconstNull,
    /// Push `int` constant -1.
    IconstM1,
    /// Push `int` constant 0.
    Iconst0,
    /// Push `int` constant 1.
    Iconst1,
    /// Push `int` constant 2.
    Iconst2,
    /// Push `int` constant 3.
    Iconst3,
    /// Push `int` constant 4.
    Iconst4,
    /// Push `int` constant 5.
    Iconst5,
    /// Push `long` constant 0.
    Lconst0,
    /// Push `long` constant 1.
    Lconst1,
    /// Push `float` constant 0.
    Fconst0,
    /// Push `float` constant 1.
    Fconst1,
    /// Push `float` constant 2.
    Fconst2,
    /// Push `double` constant 0.
    Dconst0,
    /// Push `double` constant 1.
    Dconst1,
    /// Push `byte`.
    Bipush(i8),
    /// Push `short`.
    Sipush(i16),
    /// Push item from run-time constant pool.
    Ldc(u8),
    /// Push item from run-time constant pool (wide index).
    LdcW(CpIndex),
    /// Push `long` or `double` from run-time constant pool (wide index).
    Ldc2W(CpIndex),

    // -----------------------------------------------------------------------
    // Loads
    // -----------------------------------------------------------------------
    /// Load `int` from local variable.
    Iload(u8),
    /// Load `long` from local variable.
    Lload(u8),
    /// Load `float` from local variable.
    Fload(u8),
    /// Load `double` from local variable.
    Dload(u8),
    /// Load `reference` from local variable.
    Aload(u8),
    /// Load `int` from local variable 0.
    Iload0,
    /// Load `int` from local variable 1.
    Iload1,
    /// Load `int` from local variable 2.
    Iload2,
    /// Load `int` from local variable 3.
    Iload3,
    /// Load `long` from local variable 0.
    Lload0,
    /// Load `long` from local variable 1.
    Lload1,
    /// Load `long` from local variable 2.
    Lload2,
    /// Load `long` from local variable 3.
    Lload3,
    /// Load `float` from local variable 0.
    Fload0,
    /// Load `float` from local variable 1.
    Fload1,
    /// Load `float` from local variable 2.
    Fload2,
    /// Load `float` from local variable 3.
    Fload3,
    /// Load `double` from local variable 0.
    Dload0,
    /// Load `double` from local variable 1.
    Dload1,
    /// Load `double` from local variable 2.
    Dload2,
    /// Load `double` from local variable 3.
    Dload3,
    /// Load `reference` from local variable 0.
    Aload0,
    /// Load `reference` from local variable 1.
    Aload1,
    /// Load `reference` from local variable 2.
    Aload2,
    /// Load `reference` from local variable 3.
    Aload3,
    /// Load `int` from array.
    Iaload,
    /// Load `long` from array.
    Laload,
    /// Load `float` from array.
    Faload,
    /// Load `double` from array.
    Daload,
    /// Load `reference` from array.
    Aaload,
    /// Load `byte or boolean` from array.
    Baload,
    /// Load `char` from array.
    Caload,
    /// Load `short` from array.
    Saload,

    // -----------------------------------------------------------------------
    // Stores
    // -----------------------------------------------------------------------
    /// Store `int` into local variable.
    Istore(u8),
    /// Store `long` into local variable.
    Lstore(u8),
    /// Store `float` into local variable.
    Fstore(u8),
    /// Store `double` into local variable.
    Dstore(u8),
    /// Store `reference` into local variable.
    Astore(u8),
    /// Store `int` into local variable 0.
    Istore0,
    /// Store `int` into local variable 1.
    Istore1,
    /// Store `int` into local variable 2.
    Istore2,
    /// Store `int` into local variable 3.
    Istore3,
    /// Store `long` into local variable 0.
    Lstore0,
    /// Store `long` into local variable 1.
    Lstore1,
    /// Store `long` into local variable 2.
    Lstore2,
    /// Store `long` into local variable 3.
    Lstore3,
    /// Store `float` into local variable 0.
    Fstore0,
    /// Store `float` into local variable 1.
    Fstore1,
    /// Store `float` into local variable 2.
    Fstore2,
    /// Store `float` into local variable 3.
    Fstore3,
    /// Store `double` into local variable 0.
    Dstore0,
    /// Store `double` into local variable 1.
    Dstore1,
    /// Store `double` into local variable 2.
    Dstore2,
    /// Store `double` into local variable 3.
    Dstore3,
    /// Store `reference` into local variable 0.
    Astore0,
    /// Store `reference` into local variable 1.
    Astore1,
    /// Store `reference` into local variable 2.
    Astore2,
    /// Store `reference` into local variable 3.
    Astore3,
    /// Store `int` into array.
    Iastore,
    /// Store `long` into array.
    Lastore,
    /// Store `float` into array.
    Fastore,
    /// Store `double` into array.
    Dastore,
    /// Store `reference` into array.
    Aastore,
    /// Store `byte or boolean` into array.
    Bastore,
    /// Store `char` into array.
    Castore,
    /// Store `short` into array.
    Sastore,

    // -----------------------------------------------------------------------
    // Stack
    // -----------------------------------------------------------------------
    /// Pop the top operand stack value.
    Pop,
    /// Pop the top one or two operand stack values.
    Pop2,
    /// Duplicate the top operand stack value.
    Dup,
    /// Duplicate the top operand stack value and insert two values down.
    DupX1,
    /// Duplicate the top operand stack value and insert two or three values down.
    DupX2,
    /// Duplicate the top one or two operand stack values.
    Dup2,
    /// Duplicate the top one or two operand stack values and insert two or three values down.
    Dup2X1,
    /// Duplicate the top one or two operand stack values and insert two, three, or four values down.
    Dup2X2,
    /// Swap the top two operand stack values.
    Swap,

    // -----------------------------------------------------------------------
    // Arithmetic
    // -----------------------------------------------------------------------
    /// Add `int`.
    Iadd,
    /// Add `long`.
    Ladd,
    /// Add `float`.
    Fadd,
    /// Add `double`.
    Dadd,
    /// Subtract `int`.
    Isub,
    /// Subtract `long`.
    Lsub,
    /// Subtract `float`.
    Fsub,
    /// Subtract `double`.
    Dsub,
    /// Multiply `int`.
    Imul,
    /// Multiply `long`.
    Lmul,
    /// Multiply `float`.
    Fmul,
    /// Multiply `double`.
    Dmul,
    /// Divide `int`.
    Idiv,
    /// Divide `long`.
    Ldiv,
    /// Divide `float`.
    Fdiv,
    /// Divide `double`.
    Ddiv,
    /// Remainder `int`.
    Irem,
    /// Remainder `long`.
    Lrem,
    /// Remainder `float`.
    Frem,
    /// Remainder `double`.
    Drem,
    /// Negate `int`.
    Ineg,
    /// Negate `long`.
    Lneg,
    /// Negate `float`.
    Fneg,
    /// Negate `double`.
    Dneg,
    /// Shift left `int`.
    Ishl,
    /// Shift left `long`.
    Lshl,
    /// Arithmetic shift right `int`.
    Ishr,
    /// Arithmetic shift right `long`.
    Lshr,
    /// Logical shift right `int`.
    Iushr,
    /// Logical shift right `long`.
    Lushr,
    /// Boolean AND `int`.
    Iand,
    /// Boolean AND `long`.
    Land,
    /// Boolean OR `int`.
    Ior,
    /// Boolean OR `long`.
    Lor,
    /// Boolean XOR `int`.
    Ixor,
    /// Boolean XOR `long`.
    Lxor,
    /// `iinc index const` — increment local by constant.
    Iinc {
        /// Index of local variable.
        index: u8,
        /// Value to increment by.
        value: i8,
    },

    // -----------------------------------------------------------------------
    // Conversions
    // -----------------------------------------------------------------------
    /// Convert `int` to `long`.
    I2l,
    /// Convert `int` to `float`.
    I2f,
    /// Convert `int` to `double`.
    I2d,
    /// Convert `long` to `int`.
    L2i,
    /// Convert `long` to `float`.
    L2f,
    /// Convert `long` to `double`.
    L2d,
    /// Convert `float` to `int`.
    F2i,
    /// Convert `float` to `long`.
    F2l,
    /// Convert `float` to `double`.
    F2d,
    /// Convert `double` to `int`.
    D2i,
    /// Convert `double` to `long`.
    D2l,
    /// Convert `double` to `float`.
    D2f,
    /// Convert `int` to `byte`.
    I2b,
    /// Convert `int` to `char`.
    I2c,
    /// Convert `int` to `short`.
    I2s,

    // -----------------------------------------------------------------------
    // Comparisons
    // -----------------------------------------------------------------------
    /// Compare `long`.
    Lcmp,
    /// Compare `float`.
    Fcmpl,
    /// Compare `float`.
    Fcmpg,
    /// Compare `double`.
    Dcmpl,
    /// Compare `double`.
    Dcmpg,

    // -----------------------------------------------------------------------
    // Branches — offset is relative to the PC of this instruction
    // -----------------------------------------------------------------------
    /// Branch if `int` comparison with zero succeeds.
    Ifeq(i16),
    /// Branch if `int` comparison with zero succeeds.
    Ifne(i16),
    /// Branch if `int` comparison with zero succeeds.
    Iflt(i16),
    /// Branch if `int` comparison with zero succeeds.
    Ifge(i16),
    /// Branch if `int` comparison with zero succeeds.
    Ifgt(i16),
    /// Branch if `int` comparison with zero succeeds.
    Ifle(i16),
    /// Branch if `int` comparison succeeds.
    IfIcmpeq(i16),
    /// Branch if `int` comparison succeeds.
    IfIcmpne(i16),
    /// Branch if `int` comparison succeeds.
    IfIcmplt(i16),
    /// Branch if `int` comparison succeeds.
    IfIcmpge(i16),
    /// Branch if `int` comparison succeeds.
    IfIcmpgt(i16),
    /// Branch if `int` comparison succeeds.
    IfIcmple(i16),
    /// Branch if `reference` comparison succeeds.
    IfAcmpeq(i16),
    /// Branch if `reference` comparison succeeds.
    IfAcmpne(i16),
    /// Branch always.
    Goto(i16),
    /// Jump subroutine.
    Jsr(i16),
    /// Return from subroutine.
    Ret(u8),
    /// Access jump table by index and jump.
    Tableswitch {
        /// Default offset.
        default: i32,
        /// Low bound.
        low: i32,
        /// High bound.
        high: i32,
        /// Jump offsets.
        offsets: Vec<i32>,
    },
    /// Access jump table by key match and jump.
    Lookupswitch {
        /// Default offset.
        default: i32,
        /// Pairs of (`match_value`, offset).
        pairs: Vec<(i32, i32)>,
    },

    // -----------------------------------------------------------------------
    // Returns
    // -----------------------------------------------------------------------
    /// Return from method.
    Ireturn,
    /// Return from method.
    Lreturn,
    /// Return from method.
    Freturn,
    /// Return from method.
    Dreturn,
    /// Return from method.
    Areturn,
    /// Return from method.
    Return,

    // -----------------------------------------------------------------------
    // Field access
    // -----------------------------------------------------------------------
    /// Get static field from class.
    Getstatic(CpIndex),
    /// Set static field in class.
    Putstatic(CpIndex),
    /// Fetch field from object.
    Getfield(CpIndex),
    /// Set field in object.
    Putfield(CpIndex),

    // -----------------------------------------------------------------------
    // Method invocation
    // -----------------------------------------------------------------------
    /// Invoke instance method; dispatch based on class.
    Invokevirtual(CpIndex),
    /// Invoke instance method; special handling for superclass, private, and instance initialization method invocations.
    Invokespecial(CpIndex),
    /// Invoke a class (static) method.
    Invokestatic(CpIndex),
    /// `invokeinterface index count` — `count` is the number of arguments.
    Invokeinterface {
        /// Constant pool index.
        index: CpIndex,
        /// Count of arguments.
        count: u8,
    },
    /// Invoke dynamic method.
    Invokedynamic(CpIndex),

    // -----------------------------------------------------------------------
    // Object / array
    // -----------------------------------------------------------------------
    /// Create new object.
    New(CpIndex),
    /// Create new array.
    Newarray(ArrayType),
    /// Create new array of reference.
    Anewarray(CpIndex),
    /// Get length of array.
    Arraylength,
    /// Throw exception or error.
    Athrow,
    /// Check whether object is of given type.
    Checkcast(CpIndex),
    /// Determine if object is of given type.
    Instanceof(CpIndex),
    /// Enter monitor for object.
    Monitorenter,
    /// Exit monitor for object.
    Monitorexit,

    // -----------------------------------------------------------------------
    // Extended
    // -----------------------------------------------------------------------
    /// Create new multidimensional array.
    Multianewarray {
        /// Constant pool index for class.
        index: CpIndex,
        /// Number of dimensions.
        dimensions: u8,
    },
    /// Branch if reference is null.
    Ifnull(i16),
    /// Branch if reference not null.
    Ifnonnull(i16),
    /// Branch always (wide index).
    GotoW(i32),
    /// Jump subroutine (wide index).
    JsrW(i32),

    // -----------------------------------------------------------------------
    // Wide-prefixed variants (u16 index instead of u8)
    // -----------------------------------------------------------------------
    /// Load `int` from local variable (wide index).
    IloadW(u16),
    /// Load `long` from local variable (wide index).
    LloadW(u16),
    /// Load `float` from local variable (wide index).
    FloadW(u16),
    /// Load `double` from local variable (wide index).
    DloadW(u16),
    /// Load `reference` from local variable (wide index).
    AloadW(u16),
    /// Store `int` into local variable (wide index).
    IstoreW(u16),
    /// Store `long` into local variable (wide index).
    LstoreW(u16),
    /// Store `float` into local variable (wide index).
    FstoreW(u16),
    /// Store `double` into local variable (wide index).
    DstoreW(u16),
    /// Store `reference` into local variable (wide index).
    AstoreW(u16),
    /// Return from subroutine (wide index).
    RetW(u16),
    /// Increment local variable by constant (wide index).
    IincW {
        /// Index of local variable.
        index: u16,
        /// Value to increment by.
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
    #[test]
    fn test_is_return_all() {
        let return_instructions = [
            Instruction::Ireturn,
            Instruction::Lreturn,
            Instruction::Freturn,
            Instruction::Dreturn,
            Instruction::Areturn,
            Instruction::Return,
            Instruction::Athrow,
        ];
        for instr in return_instructions {
            assert!(instr.is_return());
        }
    }

    #[test]
    fn test_control_flow_targets() {
        assert_eq!(Instruction::Return.control_flow_targets(0, Some(1)), vec![]);
        assert_eq!(Instruction::Goto(10).control_flow_targets(5, Some(8)), vec![15]);
        assert_eq!(Instruction::Jsr(10).control_flow_targets(5, Some(8)), vec![15, 8]);
        assert_eq!(Instruction::Ifeq(10).control_flow_targets(5, Some(8)), vec![15, 8]);
        assert_eq!(Instruction::Ifeq(10).control_flow_targets(5, None), vec![15]);

        assert_eq!(Instruction::Tableswitch {
            default: 10,
            low: 0,
            high: 1,
            offsets: vec![20, 30],
        }.control_flow_targets(5, Some(8)), vec![15, 25, 35]);

        assert_eq!(Instruction::Nop.control_flow_targets(5, Some(8)), vec![8]);
        assert_eq!(Instruction::Nop.control_flow_targets(5, None), vec![]);
    }
}
