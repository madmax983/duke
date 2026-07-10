//! `duke-interpreter` — The execution engine of the JVM.
//!
//! This crate orchestrates the flow of the application by interpreting decoded
//! bytecode instructions, managing threads (`threading`), handling class
//! hierarchies (`registry`), and bridging to native JNI-like functions.
//!
//! # Example
//!
//! ```
//! use duke_interpreter::{ClassRegistry, ThreadRuntime};
//!
//! let mut registry = ClassRegistry::new();
//! let mut threads = ThreadRuntime::new();
//! // Note: execution requires loaded classes and bytecode
//! ```
// proptest! macro expands to a large runner struct with no source span —
// suppress for test builds only so CI doesn't error on an un-attributable lint.
#![cfg_attr(test, allow(clippy::large_stack_arrays))]

/// Core execution context types for methods and classes.
pub(crate) mod context;
pub(crate) mod execution;
/// Repositories for loaded classes and registered native methods.
pub(crate) mod registry;
/// Core standard library classes and methods bootstrap.
pub(crate) mod stdlib;
pub(crate) mod threading;

pub use context::*;
pub use registry::*;
pub use stdlib::bootstrap_stdlib;
pub use threading::{SharedOutput, ThreadPause, ThreadRecord, ThreadRuntime};

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;

use duke_bytecode::ArrayType;
use duke_bytecode::Instruction;
use duke_classfile::CpEntry;
use duke_loader::ClassLoader;
use duke_runtime::{Error, Frame, Result, Slot};

// Registry of loaded classes — maps class name to its `ClassContext`.
//
// Used by `execute_class` for cross-class method dispatch.
//
// # Examples
//
// A native handler that can call back into the interpreter to invoke Java methods.
//
// The `invoke` closure takes `heap` and `output` as *parameters* (not captured),
// using the "loan" pattern: the handler passes its borrows through each call and
// gets them back when the call returns. Sequential reborrows — no unsafe required.
// Bootstrap minimal JDK standard library classes for native method support.
//
// Creates synthetic `java/lang/System` and `java/io/PrintStream` classes and
// registers native `println` handlers for `(Ljava/lang/String;)V`, `(I)V`,
include!("native/common.rs");
include!("native/java_lang.rs");
include!("native/java_util.rs");

// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;

#[cfg(test)]
mod fuzz;
