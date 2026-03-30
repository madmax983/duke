//! `duke-bytecode::error` — [`DecodeError`] and [`VerifyError`]

use thiserror::Error;

/// Errors produced by the bytecode decoder.
#[derive(Debug, Error)]
pub enum DecodeError {
    /// The bytecode array ended before the current instruction's operands could be fully read.
    #[error("unexpected end of bytecode at pc={pc}")]
    UnexpectedEof {
        /// The program counter where the EOF occurred.
        pc: usize,
    },

    /// The decoder encountered an unrecognized opcode byte.
    #[error("unknown opcode {opcode:#04X} at pc={pc}")]
    UnknownOpcode {
        /// The program counter of the unknown opcode.
        pc: usize,
        /// The invalid opcode byte.
        opcode: u8,
    },

    /// A `wide` (0xC4) instruction prefixed an opcode that does not support widening.
    #[error("invalid wide target opcode {opcode:#04X} at pc={pc}")]
    InvalidWideTarget {
        /// The program counter of the `wide` instruction.
        pc: usize,
        /// The invalid target opcode.
        opcode: u8,
    },

    /// A `tableswitch` instruction had a `high` value less than its `low` value.
    #[error("tableswitch at pc={pc} has high ({high}) < low ({low})")]
    InvalidTableswitch {
        /// The program counter of the instruction.
        pc: usize,
        /// The low match value.
        low: i32,
        /// The high match value.
        high: i32,
    },

    /// A `lookupswitch` instruction declared a negative number of pairs.
    #[error("lookupswitch at pc={pc} has invalid npairs ({npairs})")]
    InvalidLookupswitch {
        /// The program counter of the instruction.
        pc: usize,
        /// The declared number of pairs.
        npairs: i32,
    },

    /// A `newarray` instruction requested an array of an invalid primitive type.
    #[error("newarray at pc={pc} has invalid array type code {type_code}")]
    InvalidNewarrayType {
        /// The program counter of the instruction.
        pc: usize,
        /// The invalid primitive type code.
        type_code: u8,
    },

    /// An `invokeinterface` instruction had a non-zero fourth operand byte.
    #[error("invokeinterface at pc={pc} has non-zero reserved byte ({reserved})")]
    InvalidInvokeinterfaceReserved {
        /// The program counter of the instruction.
        pc: usize,
        /// The non-zero reserved byte.
        reserved: u8,
    },

    /// An `invokedynamic` instruction had non-zero reserved operand bytes.
    #[error("invokedynamic at pc={pc} has non-zero reserved bytes ({reserved1}, {reserved2})")]
    InvalidInvokedynamicReserved {
        /// The program counter of the instruction.
        pc: usize,
        /// The first non-zero reserved byte.
        reserved1: u8,
        /// The second non-zero reserved byte.
        reserved2: u8,
    },
}

/// Convenience alias for decoding results.
pub type DecodeResult<T> = Result<T, DecodeError>;

/// Errors produced by the structural bytecode verifier.
#[derive(Debug, Error)]
pub enum VerifyError {
    /// An instruction would push the operand stack size above `max_stack`.
    #[error("stack overflow at pc={pc}: depth would be {depth} but max_stack={max_stack}")]
    StackOverflow {
        /// The program counter of the instruction causing the overflow.
        pc: usize,
        /// The projected stack depth.
        depth: usize,
        /// The maximum allowed stack depth.
        max_stack: usize,
    },

    /// An instruction attempted to pop from an empty operand stack.
    #[error("stack underflow at pc={pc}: tried to pop from empty stack")]
    StackUnderflow {
        /// The program counter of the instruction.
        pc: usize,
    },

    /// An instruction attempted to access a local variable index `>= max_locals`.
    #[error("local variable index {index} at pc={pc} exceeds max_locals={max_locals}")]
    LocalOutOfBounds {
        /// The program counter of the instruction.
        pc: usize,
        /// The out-of-bounds local variable index.
        index: usize,
        /// The maximum number of local variables.
        max_locals: usize,
    },

    /// A return instruction was executed with a non-empty operand stack.
    #[error("non-empty stack on return at pc={pc}: {depth} value(s) remaining")]
    NonEmptyStackOnReturn {
        /// The program counter of the return instruction.
        pc: usize,
        /// The number of values left on the stack.
        depth: usize,
    },
}

/// Convenience alias for verification results.
pub type VerifyResult<T> = Result<T, VerifyError>;
