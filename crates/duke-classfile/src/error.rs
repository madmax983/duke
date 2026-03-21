//! `duke-classfile::error` — [`ParseError`] and related types

use thiserror::Error;

/// Errors that can occur while parsing a JVM `.class` file.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unexpected end of input at offset {offset}")]
    UnexpectedEof { offset: usize },

    #[error("invalid magic number: expected 0xCAFEBABE, got {got:#010X}")]
    BadMagic { got: u32 },

    #[error("unsupported class file version {major}.{minor} (Duke supports up to 65.0 = Java 21)")]
    UnsupportedVersion { major: u16, minor: u16 },

    #[error("constant pool index {index} out of bounds (pool size {pool_size})")]
    CpIndexOutOfBounds { index: u16, pool_size: usize },

    #[error("constant pool index 0 is reserved and must not be used")]
    CpIndexZero,

    #[error("constant pool slot {index} is a phantom slot (occupied by preceding Long/Double)")]
    CpPhantomSlot { index: u16 },

    #[error("unknown constant pool tag {tag} at index {index}")]
    UnknownCpTag { tag: u8, index: u16 },

    #[error("invalid CONSTANT_Utf8: {source}")]
    InvalidUtf8 {
        #[from]
        source: std::string::FromUtf8Error,
    },

    #[error("invalid method handle reference kind {kind} (must be 1–9)")]
    InvalidMethodHandleKind { kind: u8 },

    #[error("attribute length mismatch: declared {declared} bytes but consumed {consumed}")]
    AttributeLengthMismatch { declared: u32, consumed: usize },

    #[error("truncated attribute '{name}': expected {expected} bytes, got {got}")]
    TruncatedAttribute {
        name: &'static str,
        expected: usize,
        got: usize,
    },
}

/// Convenience alias.
pub type ParseResult<T> = Result<T, ParseError>;
