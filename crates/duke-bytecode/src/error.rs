use thiserror::Error;

/// Errors produced by the bytecode decoder.
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

    #[error("lookupswitch at pc={pc} has negative npairs ({npairs})")]
    InvalidLookupswitch { pc: usize, npairs: i32 },

    #[error("newarray at pc={pc} has invalid array type code {type_code}")]
    InvalidNewarrayType { pc: usize, type_code: u8 },
}

pub type DecodeResult<T> = Result<T, DecodeError>;

/// Errors produced by the structural bytecode verifier.
#[derive(Debug, Error)]
pub enum VerifyError {
    #[error(
        "stack overflow at pc={pc}: depth would be {depth} but max_stack={max_stack}"
    )]
    StackOverflow { pc: usize, depth: usize, max_stack: usize },

    #[error("stack underflow at pc={pc}: tried to pop from empty stack")]
    StackUnderflow { pc: usize },

    #[error(
        "local variable index {index} at pc={pc} exceeds max_locals={max_locals}"
    )]
    LocalOutOfBounds { pc: usize, index: usize, max_locals: usize },

    #[error("non-empty stack on return at pc={pc}: {depth} value(s) remaining")]
    NonEmptyStackOnReturn { pc: usize, depth: usize },
}

pub type VerifyResult<T> = Result<T, VerifyError>;
