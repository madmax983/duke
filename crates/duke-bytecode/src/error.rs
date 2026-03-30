//! `duke-bytecode::error` — [`DecodeError`] and [`VerifyError`]
//!
//! This module contains the canonical error types returned by the decoder
//! and structural verifier. The errors represent malformed bytecode streams
//! that violate the JVM class file specification.

use thiserror::Error;

/// Errors produced by the bytecode decoder.
///
/// These errors occur when the raw bytes of a method's `Code` attribute
/// cannot be safely transformed into a sequence of typed `Instruction`s.
///
/// # Examples
///
/// ```
/// use duke_bytecode::error::DecodeError;
///
/// let err = DecodeError::UnexpectedEof { pc: 10 };
/// assert_eq!(err.to_string(), "unexpected end of bytecode at pc=10");
/// ```
#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("unexpected end of bytecode at pc={pc}")]
    /// Expected another byte to complete an instruction, but the stream ended.
    UnexpectedEof {
        /// The program counter (byte index) where the stream ended prematurely.
        pc: usize
    },

    #[error("unknown opcode {opcode:#04X} at pc={pc}")]
    /// The decoder encountered an unrecognized opcode byte.
    UnknownOpcode {
        /// The program counter of the unknown opcode.
        pc: usize,
        /// The raw unmapped byte value.
        opcode: u8
    },

    #[error("invalid wide target opcode {opcode:#04X} at pc={pc}")]
    /// The `wide` instruction was followed by an opcode that cannot be widened.
    InvalidWideTarget {
        /// The program counter of the invalid target.
        pc: usize,
        /// The opcode that was illegally modified by `wide`.
        opcode: u8
    },

    #[error("tableswitch at pc={pc} has high ({high}) < low ({low})")]
    /// A `tableswitch` instruction's high bound was less than its low bound.
    InvalidTableswitch {
        /// The program counter of the `tableswitch` instruction.
        pc: usize,
        /// The decoded low bound.
        low: i32,
        /// The decoded high bound.
        high: i32
    },

    #[error("lookupswitch at pc={pc} has invalid npairs ({npairs})")]
    /// A `lookupswitch` instruction specified an invalid number of pairs.
    InvalidLookupswitch {
        /// The program counter of the `lookupswitch` instruction.
        pc: usize,
        /// The invalid number of jump target pairs.
        npairs: i32
    },

    #[error("newarray at pc={pc} has invalid array type code {type_code}")]
    /// A `newarray` instruction specified an unknown primitive array type code.
    InvalidNewarrayType {
        /// The program counter of the `newarray` instruction.
        pc: usize,
        /// The unknown type code (e.g., not 4-11).
        type_code: u8
    },

    #[error("invokeinterface at pc={pc} has non-zero reserved byte ({reserved})")]
    /// An `invokeinterface` instruction did not have a zero value for its reserved byte.
    InvalidInvokeinterfaceReserved {
        /// The program counter of the `invokeinterface` instruction.
        pc: usize,
        /// The non-zero reserved byte value.
        reserved: u8
    },

    #[error("invokedynamic at pc={pc} has non-zero reserved bytes ({reserved1}, {reserved2})")]
    /// An `invokedynamic` instruction did not have zero values for its reserved bytes.
    InvalidInvokedynamicReserved {
        /// The program counter of the `invokedynamic` instruction.
        pc: usize,
        /// The first non-zero reserved byte value.
        reserved1: u8,
        /// The second non-zero reserved byte value.
        reserved2: u8,
    },
}

/// Type alias for a result containing a decode error.
pub type DecodeResult<T> = Result<T, DecodeError>;

/// Errors produced by the structural bytecode verifier.
///
/// These errors occur when a decoded instruction stream is structurally
/// unsound, such as pushing more values than the maximum stack size allows
/// or popping from an empty stack.
///
/// # Examples
///
/// ```
/// use duke_bytecode::error::VerifyError;
///
/// let err = VerifyError::StackOverflow { pc: 5, depth: 3, max_stack: 2 };
/// assert_eq!(
///     err.to_string(),
///     "stack overflow at pc=5: depth would be 3 but max_stack=2"
/// );
/// ```
#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("stack overflow at pc={pc}: depth would be {depth} but max_stack={max_stack}")]
    /// An instruction pushed more values onto the stack than its allocated maximum depth.
    StackOverflow {
        /// The program counter (byte index) of the instruction that caused the overflow.
        pc: usize,
        /// The computed depth of the stack after the instruction.
        depth: usize,
        /// The maximum stack depth declared by the `Code` attribute.
        max_stack: usize,
    },

    #[error("stack underflow at pc={pc}: tried to pop from empty stack")]
    /// An instruction attempted to pop a value from an empty stack.
    StackUnderflow {
        /// The program counter (byte index) of the instruction that caused the underflow.
        pc: usize
    },

    #[error("local variable index {index} at pc={pc} exceeds max_locals={max_locals}")]
    /// An instruction attempted to access a local variable at an index greater than or equal to the maximum allowed.
    LocalOutOfBounds {
        /// The program counter (byte index) of the instruction that caused the out-of-bounds access.
        pc: usize,
        /// The invalid local variable index.
        index: usize,
        /// The maximum number of local variables declared by the `Code` attribute.
        max_locals: usize,
    },

    #[error("non-empty stack on return at pc={pc}: {depth} value(s) remaining")]
    /// A return instruction executed while there were still leftover values on the stack.
    NonEmptyStackOnReturn {
        /// The program counter (byte index) of the return instruction.
        pc: usize,
        /// The number of unexpected values left on the stack.
        depth: usize
    },
}

/// Type alias for a result containing a verification error.
pub type VerifyResult<T> = Result<T, VerifyError>;
