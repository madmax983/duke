//! `duke-bytecode::error` — [`DecodeError`] and [`VerifyError`]
//!
//! This module contains the canonical error types returned by the decoder
//! and structural verifier. The errors represent malformed bytecode streams
//! that violate the JVM class file specification.

use thiserror::Error;

/// General error enumeration encompassing both decoding and structural verification errors.
///
/// # Examples
///
/// ```
/// use duke_bytecode::error::{Error, VerifyError};
///
/// let v_err = VerifyError::StackUnderflow { pc: 10 };
/// let err: Error = v_err.into();
/// assert_eq!(err.to_string(), "Verify error: stack underflow at pc=10");
/// ```
#[derive(Debug, Error)]
pub enum Error {
    /// Wrapping a [`DecodeError`].
    #[error("Decode error: {0}")]
    Decode(#[from] DecodeError),

    /// Wrapping a [`VerifyError`].
    #[error("Verify error: {0}")]
    Verify(#[from] VerifyError),
}

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
    /// Reached end of bytecode unexpectedly.
    #[error("unexpected end of bytecode at pc={pc}")]
    UnexpectedEof {
        /// The program counter where the error occurred.
        pc: usize,
    },

    /// An unknown opcode was encountered.
    #[error("unknown opcode {opcode:#04X} at pc={pc}")]
    UnknownOpcode {
        /// The program counter where the error occurred.
        pc: usize,
        /// The unknown opcode.
        opcode: u8,
    },

    /// An invalid opcode was prefixed with `wide`.
    #[error("invalid wide target opcode {opcode:#04X} at pc={pc}")]
    InvalidWideTarget {
        /// The program counter where the error occurred.
        pc: usize,
        /// The invalid opcode.
        opcode: u8,
    },

    /// A tableswitch instruction had a high value less than its low value.
    #[error("tableswitch at pc={pc} has high ({high}) < low ({low})")]
    InvalidTableswitch {
        /// The program counter where the error occurred.
        pc: usize,
        /// The lower bound.
        low: i32,
        /// The upper bound.
        high: i32,
    },

    /// A lookupswitch instruction had an invalid number of pairs.
    #[error("lookupswitch at pc={pc} has invalid npairs ({npairs})")]
    InvalidLookupswitch {
        /// The program counter where the error occurred.
        pc: usize,
        /// The invalid number of pairs.
        npairs: i32,
    },

    /// A newarray instruction had an invalid array type code.
    #[error("newarray at pc={pc} has invalid array type code {type_code}")]
    InvalidNewarrayType {
        /// The program counter where the error occurred.
        pc: usize,
        /// The invalid type code.
        type_code: u8,
    },

    /// An invokeinterface instruction had a non-zero reserved byte.
    #[error("invokeinterface at pc={pc} has non-zero reserved byte ({reserved})")]
    InvalidInvokeinterfaceReserved {
        /// The program counter where the error occurred.
        pc: usize,
        /// The non-zero reserved byte.
        reserved: u8,
    },

    /// An invokedynamic instruction had non-zero reserved bytes.
    #[error("invokedynamic at pc={pc} has non-zero reserved bytes ({reserved1}, {reserved2})")]
    InvalidInvokedynamicReserved {
        /// The program counter where the error occurred.
        pc: usize,
        /// The first reserved byte.
        reserved1: u8,
        /// The second reserved byte.
        reserved2: u8,
    },
}

/// A generic result type for operations returning a [`DecodeError`].
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
    /// Pushing a value onto the operand stack would exceed its maximum allowed depth.
    #[error("stack overflow at pc={pc}: depth would be {depth} but max_stack={max_stack}")]
    StackOverflow {
        /// The program counter where the overflow would occur.
        pc: usize,
        /// The depth of the stack after the operation.
        depth: usize,
        /// The maximum allowed stack depth.
        max_stack: usize,
    },

    /// Popping a value from the operand stack would result in a negative depth.
    #[error("stack underflow at pc={pc}: tried to pop from empty stack")]
    StackUnderflow {
        /// The program counter where the underflow would occur.
        pc: usize,
    },

    /// Accessing a local variable at an index greater than or equal to the maximum allowed.
    #[error("local variable index {index} at pc={pc} exceeds max_locals={max_locals}")]
    LocalOutOfBounds {
        /// The program counter where the out-of-bounds access occurred.
        pc: usize,
        /// The local variable index being accessed.
        index: usize,
        /// The maximum allowed local variable index.
        max_locals: usize,
    },

    /// The operand stack is not empty when returning from a method.
    #[error("non-empty stack on return at pc={pc}: {depth} value(s) remaining")]
    NonEmptyStackOnReturn {
        /// The program counter of the return instruction.
        pc: usize,
        /// The depth of the stack when returning.
        depth: usize,
    },
}

/// A generic result type for operations returning a [`VerifyError`].
pub type VerifyResult<T> = std::result::Result<T, VerifyError>;

/// A generic result type for operations returning an [`enum@Error`].
///
/// # Examples
///
/// ```
/// use duke_bytecode::error::Result;
///
/// fn my_func() -> Result<()> {
///     Ok(())
/// }
/// ```
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let decode_err = DecodeError::UnexpectedEof { pc: 10 };
        let err: Error = decode_err.into();
        assert_eq!(
            err.to_string(),
            "Decode error: unexpected end of bytecode at pc=10"
        );

        let verify_err = VerifyError::StackUnderflow { pc: 20 };
        let err: Error = verify_err.into();
        assert_eq!(
            err.to_string(),
            "Verify error: stack underflow at pc=20: tried to pop from empty stack"
        );
    }
}
