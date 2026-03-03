use thiserror::Error;

/// Runtime errors that can occur during JVM bytecode execution.
#[derive(Debug, Error, PartialEq, Eq)]
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

    #[error("method not found: {name}{descriptor}")]
    MethodNotFound { name: String, descriptor: String },

    #[error("constant pool index {index} is not a valid Methodref")]
    InvalidMethodref { index: usize },

    #[error("null pointer dereference")]
    NullPointerException,

    #[error("invalid heap reference: address={address}")]
    InvalidRef { address: u64 },

    #[error("constant pool index {index} is not a valid Fieldref")]
    InvalidFieldref { index: usize },

    #[error("array index {index} out of bounds for length {length}")]
    ArrayIndexOutOfBounds { index: i32, length: usize },

    #[error("negative array size: {size}")]
    NegativeArraySize { size: i32 },

    #[error("java exception: {class_name}")]
    JavaException { class_name: String },

    #[error("class cast exception: {from} cannot be cast to {to}")]
    ClassCastException { from: String, to: String },

    #[error("class not found: {name}")]
    ClassNotFound { name: String },

    #[error("System.exit({code})")]
    SystemExit { code: i32 },
}

/// Convenience alias for `Result<T, VmError>`.
pub type VmResult<T> = Result<T, VmError>;
