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
/// General error enumeration that can occur when processing bytecode.
#[derive(Debug, Error)]
pub enum Error {
    /// Decode-specific error.
    #[error("Decode error: {0}")]
    Decode(#[from] DecodeError),

    /// Verify-specific error.
    #[error("Verify error: {0}")]
    Verify(#[from] VerifyError),
}

/// Errors occurring during decoding.
#[derive(Debug, Error)]
pub enum DecodeError {
    /// Encountered unexpected EOF.
    #[error("unexpected end of bytecode at pc={pc}")]
    UnexpectedEof {
        /// Current PC where EOF happened.
        pc: usize,
    },

    /// Encountered unknown opcode.
    #[error("unknown opcode {opcode:#04X} at pc={pc}")]
    UnknownOpcode {
        /// Current PC.
        pc: usize,
        /// Unknown opcode encountered.
        opcode: u8,
    },

    /// Encountered invalid target opcode after `wide`.
    #[error("invalid wide target opcode {opcode:#04X} at pc={pc}")]
    InvalidWideTarget {
        /// Current PC.
        pc: usize,
        /// The invalid opcode.
        opcode: u8,
    },

    /// `tableswitch` has invalid bounds.
    #[error("tableswitch at pc={pc} has high ({high}) < low ({low})")]
    InvalidTableswitch {
        /// Current PC.
        pc: usize,
        /// Low bound.
        low: i32,
        /// High bound.
        high: i32,
    },

    /// `lookupswitch` has invalid npairs.
    #[error("lookupswitch at pc={pc} has invalid npairs ({npairs})")]
    InvalidLookupswitch {
        /// Current PC.
        pc: usize,
        /// Invalid npairs value.
        npairs: i32,
    },

    /// `newarray` has invalid type code.
    #[error("newarray at pc={pc} has invalid array type code {type_code}")]
    InvalidNewarrayType {
        /// Current PC.
        pc: usize,
        /// Invalid type code.
        type_code: u8,
    },

    /// `invokeinterface` reserved bytes are not zero.
    #[error("invokeinterface at pc={pc} has non-zero reserved byte ({reserved})")]
    InvalidInvokeinterfaceReserved {
        /// Current PC.
        pc: usize,
        /// Non-zero reserved byte.
        reserved: u8,
    },

    /// `invokedynamic` reserved bytes are not zero.
    #[error("invokedynamic at pc={pc} has non-zero reserved bytes ({reserved1}, {reserved2})")]
    InvalidInvokedynamicReserved {
        /// Current PC.
        pc: usize,
        /// First non-zero reserved byte.
        reserved1: u8,
        /// Second non-zero reserved byte.
        reserved2: u8,
    },
}

/// Convenience alias for `Result<T, DecodeError>`.
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
    /// A stack overflow occurred during verification.
    #[error("stack overflow at pc={pc}: depth would be {depth} but max_stack={max_stack}")]
    StackOverflow {
        /// Current PC where the overflow occurred.
        pc: usize,
        /// The depth of the stack.
        depth: usize,
        /// Maximum allowed stack size.
        max_stack: usize,
    },

    /// A stack underflow occurred during verification.
    #[error("stack underflow at pc={pc}: tried to pop from empty stack")]
    StackUnderflow {
        /// Current PC where the underflow occurred.
        pc: usize,
    },

    /// An invalid local variable index was referenced.
    #[error("local variable index {index} at pc={pc} exceeds max_locals={max_locals}")]
    LocalOutOfBounds {
        /// Current PC where out of bounds happened.
        pc: usize,
        /// Local variable index referenced.
        index: usize,
        /// Maximum amount of allowed local variables.
        max_locals: usize,
    },

    /// Returning with a non-empty stack.
    #[error("non-empty stack on return at pc={pc}: {depth} value(s) remaining")]
    NonEmptyStackOnReturn {
        /// Current PC where return happened.
        pc: usize,
        /// Depth of stack values remaining.
        depth: usize,
    },
}

/// Convenience alias for `Result<T, VerifyError>`.
pub type VerifyResult<T> = std::result::Result<T, VerifyError>;

/// Convenience alias for `Result<T, Error>`.
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
