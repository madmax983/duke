//! `duke-classfile::error` — [`enum@Error`] and related types
//!
//! Why does parsing a class file fail? Because the Java Virtual Machine is a strict taskmaster.
//! When reading binary bytes, any malformed magic number, truncated byte stream, or invalid
//! constant pool index means the file is fundamentally corrupt. This module provides a detailed
//! [`enum@Error`] enumeration so that when a failure occurs, a tired developer at 3 AM knows exactly
//! *where* and *why* it happened—whether it was a missing byte or an invalid UTF-8 string.
//!
//! # Examples
//!
//! ```
//! use duke_classfile::Error;
//!
//! let err = Error::CpIndexZero;
//! assert_eq!(err.to_string(), "constant pool index 0 is reserved and must not be used");
//! ```

use thiserror::Error;

/// The grand catalog of everything that can go wrong when reading a JVM `.class` file.
///
/// We don't just return a generic "parse failed" message. We want you to know the exact
/// offset of the truncation, the specific invalid magic number, or the out-of-bounds
/// [`crate::constant_pool::CpIndex`]. Use these variants to log precise, helpful diagnostics.
///
/// # Examples
///
/// ```
/// use duke_classfile::Error;
///
/// let err = Error::UnexpectedEof { offset: 42 };
/// match err {
///     Error::UnexpectedEof { offset } => assert_eq!(offset, 42),
///     _ => panic!("Expected UnexpectedEof"),
/// }
/// ```
#[derive(Debug, Error)]
pub enum Error {

    /// A recursive structure (like nested annotations) exceeded the maximum depth limit.
    #[error("recursion limit exceeded while parsing class file")]
    RecursionLimitExceeded,

    /// Expected more bytes to parse but reached the end of the file.
    #[error("unexpected end of input at offset {offset}")]
    UnexpectedEof {
        /// Offset where the EOF occurred.
        offset: usize,
    },

    /// The parsed file does not begin with the JVM magic number (`0xCAFEBABE`).
    #[error("invalid magic number: expected 0xCAFEBABE, got {got:#010X}")]
    BadMagic {
        /// The magic number that was read.
        got: u32,
    },

    /// The class file specifies a version not supported by this runtime.
    #[error("unsupported class file version {major}.{minor} (Duke supports up to 65.0 = Java 21)")]
    UnsupportedVersion {
        /// Major version of the class file.
        major: u16,
        /// Minor version of the class file.
        minor: u16,
    },

    /// A constant pool index refers to an invalid location.
    #[error("constant pool index {index} out of bounds (pool size {pool_size})")]
    CpIndexOutOfBounds {
        /// The invalid constant pool index.
        index: u16,
        /// Total size of the constant pool.
        pool_size: usize,
    },

    /// Constant pool index 0 is explicitly reserved and invalid for normal use.
    #[error("constant pool index 0 is reserved and must not be used")]
    CpIndexZero,

    /// A reference attempted to read the second (phantom) slot of a `Long` or `Double` entry.
    #[error("constant pool slot {index} is a phantom slot (occupied by preceding Long/Double)")]
    CpPhantomSlot {
        /// Index of the phantom slot.
        index: u16,
    },

    /// Encountered an unknown or unsupported constant pool tag.
    #[error("unknown constant pool tag {tag} at index {index}")]
    UnknownCpTag {
        /// The invalid tag byte.
        tag: u8,
        /// Index in the constant pool where the tag was encountered.
        index: u16,
    },

    /// The bytes for a `CONSTANT_Utf8_info` entry are not valid UTF-8.
    #[error("invalid CONSTANT_Utf8: {source}")]
    InvalidUtf8 {
        /// The underlying UTF-8 error.
        #[from]
        source: std::string::FromUtf8Error,
    },

    /// An invalid reference kind was provided in a `MethodHandle` structure.
    #[error("invalid method handle reference kind {kind} (must be 1–9)")]
    InvalidMethodHandleKind {
        /// The invalid reference kind.
        kind: u8,
    },

    /// The attribute length declared does not match the amount of data successfully parsed.
    #[error("attribute length mismatch: declared {declared} bytes but consumed {consumed}")]
    AttributeLengthMismatch {
        /// Expected length according to the class file.
        declared: u32,
        /// The amount actually parsed.
        consumed: usize,
    },

    /// An attribute ended prematurely.
    #[error("truncated attribute '{name}': expected {expected} bytes, got {got}")]
    TruncatedAttribute {
        /// The name of the attribute.
        name: &'static str,
        /// Expected number of bytes remaining.
        expected: usize,
        /// Actual number of bytes available.
        got: usize,
    },

    /// An annotation element-value tag was unknown.
    #[error("invalid annotation element-value tag {tag:#04X}")]
    InvalidAnnotationElementValueTag {
        /// Raw tag byte.
        tag: u8,
    },
}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, Error>;
