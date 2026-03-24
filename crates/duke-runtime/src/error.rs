//! `duke-runtime::error` — Runtime errors
//!
//! This module defines the [`VmError`] enum, which represents all possible failure modes
//! during JVM bytecode execution within a specific [`crate::Frame`].
//!
//! The JVM execution state revolves around manipulating data stored in [`crate::Slot`]s.
//! Many errors defined here correspond to invalid manipulation of these slots, such as
//! popping from an empty stack ([`VmError::StackUnderflow`]), pushing past the maximum
//! frame capacity ([`VmError::StackOverflow`]), or expecting an `int` slot but finding
//! a `float` slot ([`VmError::TypeMismatch`]).

use thiserror::Error;

/// Runtime errors that can occur during JVM bytecode execution.
///
/// # Examples
///
/// Demonstrating how to instantiate and format the various error types:
///
/// ```
/// use duke_runtime::VmError;
///
/// // Stack operations
/// assert_eq!(VmError::StackOverflow.to_string(), "operand stack overflow");
/// assert_eq!(VmError::StackUnderflow.to_string(), "operand stack underflow");
///
/// // Local variables
/// assert_eq!(
///     VmError::LocalOutOfBounds { index: 5, max_locals: 3 }.to_string(),
///     "local variable index 5 out of bounds (max_locals=3)"
/// );
///
/// // Math and Execution Control
/// assert_eq!(VmError::DivisionByZero.to_string(), "integer division by zero");
/// assert_eq!(VmError::InvalidBranchTarget { pc: 100 }.to_string(), "invalid branch target: pc=100");
/// assert_eq!(VmError::FellOffEnd.to_string(), "fell off end of bytecode without a return instruction");
/// assert_eq!(VmError::SystemExit { code: 1 }.to_string(), "System.exit(1)");
///
/// // Type and Validation Errors
/// assert_eq!(
///     VmError::TypeMismatch { expected: "int", got: "float" }.to_string(),
///     "type mismatch: expected int, got float"
/// );
/// assert_eq!(
///     VmError::Unimplemented { mnemonic: "invoke_dynamic" }.to_string(),
///     "unimplemented instruction: invoke_dynamic"
/// );
/// assert_eq!(VmError::InvalidCpIndex { index: 42 }.to_string(), "invalid constant pool index 42");
/// assert_eq!(VmError::InvalidMethodref { index: 12 }.to_string(), "constant pool index 12 is not a valid Methodref");
/// assert_eq!(VmError::InvalidFieldref { index: 9 }.to_string(), "constant pool index 9 is not a valid Fieldref");
///
/// // Resolution Errors
/// assert_eq!(
///     VmError::MethodNotFound { name: "foo".into(), descriptor: "()V".into() }.to_string(),
///     "method not found: foo()V"
/// );
/// assert_eq!(
///     VmError::ClassNotFound { name: "java/lang/Missing".into() }.to_string(),
///     "class not found: java/lang/Missing"
/// );
///
/// // Object and Memory Errors
/// assert_eq!(VmError::NullPointerException.to_string(), "null pointer dereference");
/// assert_eq!(VmError::InvalidRef { address: 0xDEADBEEF }.to_string(), "invalid heap reference: address=3735928559");
/// assert_eq!(
///     VmError::ArrayIndexOutOfBounds { index: 5, length: 3 }.to_string(),
///     "array index 5 out of bounds for length 3"
/// );
/// assert_eq!(VmError::NegativeArraySize { size: -1 }.to_string(), "negative array size: -1");
///
/// // Java Specific Errors
/// assert_eq!(
///     VmError::JavaException { class_name: "java/lang/RuntimeException".into() }.to_string(),
///     "java exception: java/lang/RuntimeException"
/// );
/// assert_eq!(
///     VmError::ClassCastException { from: "java/lang/Object".into(), to: "java/lang/String".into() }.to_string(),
///     "class cast exception: java/lang/Object cannot be cast to java/lang/String"
/// );
/// assert_eq!(
///     VmError::InstantiationError { class_name: "java/lang/Number".into() }.to_string(),
///     "InstantiationError: cannot instantiate abstract class java/lang/Number"
/// );
/// assert_eq!(
///     VmError::AbstractMethodError { class_name: "java/lang/Number".into(), method_name: "intValue".into() }.to_string(),
///     "AbstractMethodError: java/lang/Number.intValue"
/// );
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
