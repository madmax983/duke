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
use duke_classfile::types::CpEntry;
use duke_loader::ClassLoader;
use duke_runtime::{Error, Frame, Result, Slot};

use std::sync::{RwLock, OnceLock};
use std::sync::atomic::{AtomicI32, Ordering};


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
include!("native/allocate.rs");
include!("native/annotation.rs");
include!("native/arraydeque.rs");
include!("native/arraylist.rs");
include!("native/arrays.rs");
include!("native/atomic.rs");
include!("native/base64.rs");
include!("native/boot.rs");
include!("native/byte.rs");
include!("native/char.rs");
include!("native/charset.rs");
include!("native/class.rs");
include!("native/collections.rs");
include!("native/collectors.rs");
include!("native/comparator.rs");
include!("native/concurrent.rs");
include!("native/condition.rs");
include!("native/core.rs");
include!("native/count.rs");
include!("native/cyclic.rs");
include!("native/decode.rs");
include!("native/double.rs");
include!("native/duration.rs");
include!("native/executor.rs");
include!("native/extract.rs");
include!("native/file.rs");
include!("native/future.rs");
include!("native/hashmap.rs");
include!("native/hashset.rs");
include!("native/instant.rs");
include!("native/int.rs");
include!("native/integer.rs");
include!("native/is.rs");
include!("native/jul.rs");
include!("native/linked.rs");
include!("native/localdate.rs");
include!("native/localdatetime.rs");
include!("native/long.rs");
include!("native/lookup.rs");
include!("native/make.rs");
include!("native/map.rs");
include!("native/matcher.rs");
include!("native/math.rs");
include!("native/message.rs");
include!("native/object.rs");
include!("native/objects.rs");
include!("native/optional.rs");
include!("native/parse.rs");
include!("native/pattern.rs");
include!("native/period.rs");
include!("native/print.rs");
include!("native/println.rs");
include!("native/priorityqueue.rs");
include!("native/process.rs");
include!("native/properties.rs");
include!("native/provider.rs");
include!("native/push.rs");
include!("native/random.rs");
include!("native/read.rs");
include!("native/reentrant.rs");
include!("native/reflect.rs");
include!("native/reflection.rs");
include!("native/regex.rs");
include!("native/resolve.rs");
include!("native/resource.rs");
include!("native/runtime.rs");
include!("native/sb.rs");
include!("native/semaphore.rs");
include!("native/service.rs");
include!("native/short.rs");
include!("native/stack.rs");
include!("native/stream.rs");
include!("native/string.rs");
include!("native/stringbuffer.rs");
include!("native/stringjoiner.rs");
include!("native/system.rs");
include!("native/tests_sentry.rs");
include!("native/thread.rs");
include!("native/throwable.rs");
include!("native/treemap.rs");
include!("native/treeset.rs");
include!("native/url.rs");
include!("native/uuid.rs");
include!("native/with.rs");
include!("native/write.rs");
include!("native/zip.rs");

// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;

#[cfg(test)]
mod fuzz;
