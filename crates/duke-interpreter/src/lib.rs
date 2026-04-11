//! Switch-dispatch JVM bytecode interpreter for Duke Phase 4.
//!
//! Executes decoded instruction streams for methods containing integer, long,
//! float, and double arithmetic, control flow, and local variables.  Heap
//! allocation, field access, and method invocation are not yet implemented.
// proptest! macro expands to a large runner struct with no source span —
// suppress for test builds only so CI doesn't error on an un-attributable lint.
#![cfg_attr(test, allow(clippy::large_stack_arrays))]

/// Core execution context types for methods and classes.
pub mod context;
pub(crate) mod execution;
/// Repositories for loaded classes and registered native methods.
pub mod registry;
/// Core standard library classes and methods bootstrap.
pub mod stdlib;
pub mod threading;

pub use context::*;
pub use registry::*;
pub use stdlib::bootstrap_stdlib;

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;

use duke_bytecode::Instruction;
use duke_bytecode::instruction::ArrayType;
use duke_classfile::types::CpEntry;
use duke_loader::ClassLoader;
use duke_runtime::{Frame, Slot, VmError, VmResult};

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
include!("native.rs");

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    include!("tests.rs");
}
#[cfg(test)]
mod fuzz;
