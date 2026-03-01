use thiserror::Error;

/// Runtime errors that can occur during JVM bytecode execution.
#[derive(Debug, Error, PartialEq)]
pub enum VmError {
    #[error("operand stack overflow")]
    StackOverflow,

    #[error("operand stack underflow")]
    StackUnderflow,

    #[error("local variable index {index} out of bounds (max_locals={max_locals})")]
    LocalOutOfBounds { index: usize, max_locals: usize },

    #[error("integer division by zero")]
    DivisionByZero,

    #[error("invalid branch target: pc={pc}")]
    InvalidBranchTarget { pc: usize },

    #[error("fell off end of bytecode without a return instruction")]
    FellOffEnd,

    #[error("type mismatch: expected {expected}, got {got}")]
    TypeMismatch {
        expected: &'static str,
        got: &'static str,
    },

    #[error("unimplemented instruction: {mnemonic}")]
    Unimplemented { mnemonic: &'static str },

    #[error("invalid constant pool index {index}")]
    InvalidCpIndex { index: usize },
}

/// Convenience alias for `Result<T, VmError>`.
pub type VmResult<T> = Result<T, VmError>;
