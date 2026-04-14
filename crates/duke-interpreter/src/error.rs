//! `duke-interpreter::error` — Interpreter error types.

/// The standard error type for the Duke Interpreter, wrapping runtime errors.
pub type Error = duke_runtime::VmError;

/// The standard result type for the Duke Interpreter.
pub type Result<T> = std::result::Result<T, Error>;
