//! `duke-runtime::error` — Runtime errors

use thiserror::Error;

/// Runtime errors that can occur during JVM bytecode execution.
///
/// # Examples
///
/// ```
/// use duke_runtime::VmError;
///
/// let err = VmError::NullPointerException;
/// assert_eq!(err.to_string(), "null pointer dereference");
///
/// let err = VmError::DivisionByZero;
/// assert_eq!(err.to_string(), "integer division by zero");
/// ```
#[derive(Debug, Error, PartialEq, Eq)]
pub enum VmError {
    /// Pushed more items onto the stack than its `max_stack` allows.
    #[error("operand stack overflow")]
    StackOverflow,

    /// Popped an item from an empty operand stack.
    #[error("operand stack underflow")]
    StackUnderflow,

    /// Tried to access a local variable beyond `max_locals`.
    #[error("local variable index {index} out of bounds (max_locals={max_locals})")]
    LocalOutOfBounds {
        /// The index that was attempted to be accessed.
        index: usize,
        /// The maximum number of locals defined for the method.
        max_locals: usize,
    },

    /// A division or remainder operation by zero.
    #[error("integer division by zero")]
    DivisionByZero,

    /// A branch target was outside the valid code range.
    #[error("invalid branch target: pc={pc}")]
    InvalidBranchTarget {
        /// The program counter that was computed.
        pc: usize,
    },

    /// Execution passed the end of the `code` array without returning or throwing an exception.
    #[error("fell off end of bytecode without a return instruction")]
    FellOffEnd,

    /// Expected a different JVM type than what was found.
    #[error("type mismatch: expected {expected}, got {got}")]
    TypeMismatch {
        /// The expected type.
        expected: &'static str,
        /// The actual type encountered.
        got: &'static str,
    },

    /// Encountered an instruction that is not implemented yet.
    #[error("unimplemented instruction: {mnemonic}")]
    Unimplemented {
        /// The mnemonic of the unimplemented instruction.
        mnemonic: &'static str,
    },

    /// A constant pool index was out of bounds or pointed to a missing entry.
    #[error("invalid constant pool index {index}")]
    InvalidCpIndex {
        /// The invalid constant pool index.
        index: usize,
    },

    /// Method resolution failed to find a matching method.
    #[error("method not found: {name}{descriptor}")]
    MethodNotFound {
        /// The name of the missing method.
        name: String,
        /// The descriptor of the missing method.
        descriptor: String,
    },

    /// An index did not resolve to a `Methodref` or `InterfaceMethodref`.
    #[error("constant pool index {index} is not a valid Methodref")]
    InvalidMethodref {
        /// The constant pool index that caused the error.
        index: usize,
    },

    /// An operation attempted to dereference a null reference.
    #[error("null pointer dereference")]
    NullPointerException,

    /// A reference points to an unallocated or invalid heap location.
    #[error("invalid heap reference: address={address}")]
    InvalidRef {
        /// The invalid heap address.
        address: u64,
    },

    /// An index did not resolve to a `Fieldref`.
    #[error("constant pool index {index} is not a valid Fieldref")]
    InvalidFieldref {
        /// The constant pool index that caused the error.
        index: usize,
    },

    /// An array access was out of bounds.
    #[error("array index {index} out of bounds for length {length}")]
    ArrayIndexOutOfBounds {
        /// The index attempted to be accessed.
        index: i32,
        /// The length of the array.
        length: usize,
    },

    /// Attempted to create an array with a negative length.
    #[error("negative array size: {size}")]
    NegativeArraySize {
        /// The negative size.
        size: i32,
    },

    /// A Java exception was thrown.
    #[error("java exception: {class_name}")]
    JavaException {
        /// The class name of the thrown exception.
        class_name: String,
    },

    /// A type cast failed during execution.
    #[error("class cast exception: {from} cannot be cast to {to}")]
    ClassCastException {
        /// The source type class name.
        from: String,
        /// The target type class name.
        to: String,
    },

    /// Class loading failed to locate a requested class.
    #[error("class not found: {name}")]
    ClassNotFound {
        /// The name of the missing class.
        name: String,
    },

    /// A plain class name matched multiple loaded definitions from different loaders.
    #[error("ambiguous class name: {name} matches {matches:?}")]
    AmbiguousClassName {
        /// The ambiguous binary/internal name that was requested.
        name: String,
        /// The exact loaded class keys that matched.
        matches: Vec<String>,
    },

    /// `System.exit()` was called, signaling VM termination.
    #[error("System.exit({code})")]
    SystemExit {
        /// The exit code provided to `System.exit`.
        code: i32,
    },

    /// Attempted to instantiate an abstract class or interface.
    #[error("InstantiationError: cannot instantiate abstract class {class_name}")]
    InstantiationError {
        /// The name of the abstract class or interface.
        class_name: String,
    },

    /// Attempted to invoke an abstract method.
    #[error("AbstractMethodError: {class_name}.{method_name}")]
    AbstractMethodError {
        /// The class name in which the invocation occurred.
        class_name: String,
        /// The method name.
        method_name: String,
    },
}

/// Convenience alias for `Result<T, VmError>`.
///
/// # Examples
///
/// ```
/// use duke_runtime::{VmError, VmResult};
///
/// fn might_fail(fail: bool) -> VmResult<i32> {
///     if fail {
///         Err(VmError::StackUnderflow)
///     } else {
///         Ok(42)
///     }
/// }
///
/// assert!(might_fail(true).is_err());
/// assert_eq!(might_fail(false).unwrap(), 42);
/// ```
pub type VmResult<T> = Result<T, VmError>;
