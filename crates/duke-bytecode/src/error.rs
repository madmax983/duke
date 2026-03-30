//! `duke-bytecode::error` — [`DecodeError`] and [`VerifyError`]

use thiserror::Error;

/// Errors produced by the bytecode decoder.
#[derive(Debug, Error)]
pub enum DecodeError {
    /// The decoder ran out of bytes while reading an instruction's operands.
    #[error("unexpected end of bytecode at pc={pc}")]
    UnexpectedEof {
        /// The program counter where the EOF was encountered.
        pc: usize,
    },

    /// The decoder encountered an unrecognized opcode byte.
    #[error("unknown opcode {opcode:#04X} at pc={pc}")]
    UnknownOpcode {
        /// The program counter where the unknown opcode was found.
        pc: usize,
        /// The unrecognized opcode byte.
        opcode: u8,
    },

    /// The decoder encountered a `wide` prefix followed by an invalid target instruction.
    #[error("invalid wide target opcode {opcode:#04X} at pc={pc}")]
    InvalidWideTarget {
        /// The program counter of the `wide` instruction.
        pc: usize,
        /// The invalid target opcode that followed the `wide` instruction.
        opcode: u8,
    },

    /// The `tableswitch` instruction has a `high` value that is less than its `low` value.
    #[error("tableswitch at pc={pc} has high ({high}) < low ({low})")]
    InvalidTableswitch {
        /// The program counter of the `tableswitch` instruction.
        pc: usize,
        /// The minimum index (inclusive) of the jump table.
        low: i32,
        /// The maximum index (inclusive) of the jump table.
        high: i32,
    },

    /// The `lookupswitch` instruction has an invalid or negative number of pairs.
    #[error("lookupswitch at pc={pc} has invalid npairs ({npairs})")]
    InvalidLookupswitch {
        /// The program counter of the `lookupswitch` instruction.
        pc: usize,
        /// The invalid number of pairs.
        npairs: i32,
    },

    /// The `newarray` instruction specifies an unknown primitive array type code.
    #[error("newarray at pc={pc} has invalid array type code {type_code}")]
    InvalidNewarrayType {
        /// The program counter of the `newarray` instruction.
        pc: usize,
        /// The unknown primitive array type code.
        type_code: u8,
    },

    /// The `invokeinterface` instruction does not have a zero value for its reserved fourth byte.
    #[error("invokeinterface at pc={pc} has non-zero reserved byte ({reserved})")]
    InvalidInvokeinterfaceReserved {
        /// The program counter of the `invokeinterface` instruction.
        pc: usize,
        /// The invalid reserved byte value.
        reserved: u8,
    },

    /// The `invokedynamic` instruction does not have zero values for its reserved third and fourth bytes.
    #[error("invokedynamic at pc={pc} has non-zero reserved bytes ({reserved1}, {reserved2})")]
    InvalidInvokedynamicReserved {
        /// The program counter of the `invokedynamic` instruction.
        pc: usize,
        /// The first invalid reserved byte.
        reserved1: u8,
        /// The second invalid reserved byte.
        reserved2: u8,
    },
}

/// The result type for bytecode decoding operations.
pub type DecodeResult<T> = Result<T, DecodeError>;

/// Errors produced by the structural bytecode verifier.
#[derive(Debug, Error)]
pub enum VerifyError {
    /// A push instruction would exceed the method's maximum operand stack depth.
    #[error("stack overflow at pc={pc}: depth would be {depth} but max_stack={max_stack}")]
    StackOverflow {
        /// The program counter of the instruction that caused the overflow.
        pc: usize,
        /// The new stack depth after the instruction.
        depth: usize,
        /// The maximum allowed stack depth.
        max_stack: usize,
    },

    /// A pop instruction attempted to remove values from an empty operand stack.
    #[error("stack underflow at pc={pc}: tried to pop from empty stack")]
    StackUnderflow {
        /// The program counter of the instruction that caused the underflow.
        pc: usize,
    },

    /// An instruction attempted to access a local variable index outside the allowed range.
    #[error("local variable index {index} at pc={pc} exceeds max_locals={max_locals}")]
    LocalOutOfBounds {
        /// The program counter of the instruction that caused the out-of-bounds access.
        pc: usize,
        /// The local variable index being accessed.
        index: usize,
        /// The maximum allowed local variable index.
        max_locals: usize,
    },

    /// A return instruction executed with remaining items on the operand stack.
    #[error("non-empty stack on return at pc={pc}: {depth} value(s) remaining")]
    NonEmptyStackOnReturn {
        /// The program counter of the return instruction.
        pc: usize,
        /// The number of values remaining on the stack.
        depth: usize,
    },
}

/// The result type for structural bytecode verification operations.
pub type VerifyResult<T> = Result<T, VerifyError>;
