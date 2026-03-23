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
pub enum Error {
    #[error("Decode error: {0}")]
    Decode(#[from] DecodeError),

    #[error("Verify error: {0}")]
    Verify(#[from] VerifyError),
}

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("unexpected end of bytecode at pc={pc}")]
    UnexpectedEof { pc: usize },

    #[error("unknown opcode {opcode:#04X} at pc={pc}")]
    UnknownOpcode { pc: usize, opcode: u8 },

    #[error("invalid wide target opcode {opcode:#04X} at pc={pc}")]
    InvalidWideTarget { pc: usize, opcode: u8 },

    #[error("tableswitch at pc={pc} has high ({high}) < low ({low})")]
    InvalidTableswitch { pc: usize, low: i32, high: i32 },

    #[error("lookupswitch at pc={pc} has invalid npairs ({npairs})")]
    InvalidLookupswitch { pc: usize, npairs: i32 },

    #[error("newarray at pc={pc} has invalid array type code {type_code}")]
    InvalidNewarrayType { pc: usize, type_code: u8 },

    #[error("invokeinterface at pc={pc} has non-zero reserved byte ({reserved})")]
    InvalidInvokeinterfaceReserved { pc: usize, reserved: u8 },

    #[error("invokedynamic at pc={pc} has non-zero reserved bytes ({reserved1}, {reserved2})")]
    InvalidInvokedynamicReserved {
        pc: usize,
        reserved1: u8,
        reserved2: u8,
    },
}

pub type DecodeResult<T> = std::result::Result<T, DecodeError>;

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
    StackOverflow {
        pc: usize,
        depth: usize,
        max_stack: usize,
    },

    #[error("stack underflow at pc={pc}: tried to pop from empty stack")]
    StackUnderflow { pc: usize },

    #[error("local variable index {index} at pc={pc} exceeds max_locals={max_locals}")]
    LocalOutOfBounds {
        pc: usize,
        index: usize,
        max_locals: usize,
    },

    #[error("non-empty stack on return at pc={pc}: {depth} value(s) remaining")]
    NonEmptyStackOnReturn { pc: usize, depth: usize },
}

pub type VerifyResult<T> = std::result::Result<T, VerifyError>;
pub type Result<T> = std::result::Result<T, Error>;
