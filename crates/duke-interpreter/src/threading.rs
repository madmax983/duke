//! `duke-interpreter::threading` — Thread management
//!
//! This module provides the internal execution scaffold for handling Java threads
//! within the interpreter. Because Duke runs as a single-threaded execution loop
//! at its core, it requires a way to track, suspend, and synchronize independent
//! Java execution contexts without natively blocking the Rust host thread.
//!
//! The structures here (like `ThreadRuntime` and `SharedOutput`) allow native methods
//! to register new threads and coordinate standard output safely across execution frames.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Marker for a Java thread action that the runtime can pause on later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadPause {
    /// Instructs the current Java thread to sleep for the specified duration.
    Sleep(Duration),
    /// Instructs the current Java thread to wait until another thread finishes.
    Join {
        /// The internal runtime ID of the target thread to wait for.
        thread_id: i32,
    },
}

/// Shared output sink placeholder for future threaded interpreter writes.
#[derive(Debug, Clone, Default)]
pub struct SharedOutput {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl SharedOutput {
    /// Creates a new `SharedOutput` instance with an empty buffer.
    ///
    /// This is used when a background Java thread needs to write to `System.out`
    /// or `System.err`. By allocating a fresh buffer, we avoid interleaving
    /// byte streams directly to the host's stdout until the interpreter loop
    /// decides it is safe to flush.
    ///
    /// ## Examples
    ///
    /// ```
    /// use duke_interpreter::SharedOutput;
    /// let output = SharedOutput::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Wraps an existing shared buffer into a `SharedOutput` instance.
    ///
    /// This is useful when you already have a byte buffer (perhaps passed in from
    /// a different subsystem) and need to attach it to the threading machinery
    /// for Java output redirection.
    ///
    /// ## Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// use duke_interpreter::SharedOutput;
    /// let buffer = Arc::new(Mutex::new(Vec::new()));
    /// let output = SharedOutput::from_buffer(buffer);
    /// ```
    #[must_use]
    pub const fn from_buffer(buffer: Arc<Mutex<Vec<u8>>>) -> Self {
        Self { buffer }
    }

    /// Extracts a thread-safe, clonable reference to the underlying output buffer.
    ///
    /// The buffer is wrapped in an `Arc<Mutex<..>>` because native handlers
    /// and the interpreter loop both need to push and drain bytes concurrently
    /// without violating Rust's aliasing rules.
    ///
    /// ## Examples
    ///
    /// ```
    /// use duke_interpreter::SharedOutput;
    /// let output = SharedOutput::new();
    /// let buffer = output.buffer();
    /// ```
    #[must_use]
    pub fn buffer(&self) -> Arc<Mutex<Vec<u8>>> {
        Arc::clone(&self.buffer)
    }
}

/// Minimal metadata for a Java thread record.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThreadRecord {
    /// The heap reference (object ID) of the underlying `java/lang/Thread` instance.
    pub java_ref: u64,
    /// The internal runtime thread identifier.
    pub thread_id: i32,
    /// Indicates whether the thread has completed execution.
    pub finished: bool,
    /// Indicates whether the thread is a daemon thread.
    /// The JVM will exit when only daemon threads remain.
    pub daemon: bool,
}

impl ThreadRecord {
    /// Constructs a new active `ThreadRecord`.
    ///
    /// The thread is marked as unfinished and non-daemon by default.
    ///
    /// ## Examples
    ///
    /// ```
    /// use duke_interpreter::ThreadRecord;
    /// let record = ThreadRecord::new(12345, 1);
    /// assert_eq!(record.java_ref, 12345);
    /// assert_eq!(record.thread_id, 1);
    /// ```
    #[must_use]
    pub const fn new(java_ref: u64, thread_id: i32) -> Self {
        Self {
            java_ref,
            thread_id,
            finished: false,
            daemon: false,
        }
    }

    /// Mark this record as finished and report whether the state changed.
    pub const fn mark_finished(&mut self) -> bool {
        if self.finished {
            return false;
        }
        self.finished = true;
        true
    }
}

/// Interpreter-owned threading runtime scaffold.
#[derive(Debug, Default)]
pub struct ThreadRuntime {
    next_thread_id: i32,
    records: Vec<ThreadRecord>,
}

impl ThreadRuntime {
    /// Allocates a new empty `ThreadRuntime`.
    ///
    /// The runtime begins at thread ID 0 and holds no active worker records.
    /// This is typically instantiated once per JVM launch to act as the global
    /// scoreboard for active threads.
    ///
    /// ## Examples
    ///
    /// ```
    /// use duke_interpreter::ThreadRuntime;
    /// let runtime = ThreadRuntime::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Peeks at the next available thread identifier without allocating it.
    ///
    /// ## Examples
    ///
    /// ```
    /// use duke_interpreter::ThreadRuntime;
    /// let runtime = ThreadRuntime::new();
    /// assert_eq!(runtime.next_thread_id(), 0);
    /// ```
    #[must_use]
    pub const fn next_thread_id(&self) -> i32 {
        self.next_thread_id
    }

    /// Calculates the number of currently active, unfinished Java threads.
    ///
    /// The JVM specification dictates that the virtual machine cannot exit
    /// until all non-daemon threads have completed. This method provides the
    /// interpreter loop with that termination signal.
    ///
    /// ## Examples
    ///
    /// ```
    /// use duke_interpreter::{ThreadRuntime, ThreadRecord};
    /// let mut runtime = ThreadRuntime::new();
    /// runtime.register(ThreadRecord::new(123, 0));
    /// assert_eq!(runtime.live_workers(), 1);
    /// ```
    #[must_use]
    pub fn live_workers(&self) -> usize {
        self.records
            .iter()
            .filter(|record| !record.finished)
            .count()
    }

    /// Exposes a read-only view of all recorded Java threads.
    ///
    /// This is used primarily by garbage collection to scan active threads
    /// for root object references to prevent premature reclamation.
    ///
    /// ## Examples
    ///
    /// ```
    /// use duke_interpreter::ThreadRuntime;
    /// let runtime = ThreadRuntime::new();
    /// assert_eq!(runtime.records().len(), 0);
    /// ```
    #[must_use]
    pub fn records(&self) -> &[ThreadRecord] {
        &self.records
    }

    /// Retrieves the next available thread identifier and advances the counter.
    ///
    /// This ensures that every newly created Java thread receives a unique ID
    /// within the `ThreadRuntime`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use duke_interpreter::ThreadRuntime;
    /// let mut runtime = ThreadRuntime::new();
    /// let id1 = runtime.allocate_thread_id();
    /// let id2 = runtime.allocate_thread_id();
    /// assert_eq!(id1, 0);
    /// assert_eq!(id2, 1);
    /// ```
    #[must_use]
    pub const fn allocate_thread_id(&mut self) -> i32 {
        let id = self.next_thread_id;
        self.next_thread_id = self.next_thread_id.wrapping_add(1);
        id
    }

    /// Injects a newly allocated Java thread into the interpreter's lifecycle manager.
    ///
    /// Without registering the thread record, the interpreter loop will not track
    /// its termination, potentially causing the JVM to exit prematurely or leak resources.
    ///
    /// ## Examples
    ///
    /// ```
    /// use duke_interpreter::{ThreadRuntime, ThreadRecord};
    /// let mut runtime = ThreadRuntime::new();
    /// runtime.register(ThreadRecord::new(123, 0));
    /// ```
    pub fn register(&mut self, record: ThreadRecord) {
        self.records.push(record);
    }

    /// Flags a worker thread as completed using its underlying native OS ID.
    ///
    /// When a native thread executing a Java task completes, it must signal
    /// the runtime so that `live_workers` can decrement, unblocking `Thread.join()`
    /// calls and potentially allowing the JVM to shut down safely.
    ///
    /// Returns `true` if the thread was found and marked as finished; `false` otherwise.
    ///
    /// ## Examples
    ///
    /// ```
    /// use duke_interpreter::{ThreadRuntime, ThreadRecord};
    /// let mut runtime = ThreadRuntime::new();
    /// runtime.register(ThreadRecord::new(123, 0));
    /// assert!(runtime.mark_finished(0));
    /// ```
    #[must_use]
    pub fn mark_finished(&mut self, thread_id: i32) -> bool {
        let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.thread_id == thread_id)
        else {
            return false;
        };
        record.mark_finished()
    }

    /// Flags a worker thread as completed using its JVM internal `java/lang/Thread` object reference.
    ///
    /// Similar to `mark_finished`, but operates on the heap reference rather than
    /// the native ID. This is heavily utilized when resolving `Thread.join()` invocations,
    /// where the caller only holds a reference to the target Java thread object.
    ///
    /// Returns `true` if the thread was found and marked as finished; `false` otherwise.
    ///
    /// ## Examples
    ///
    /// ```
    /// use duke_interpreter::{ThreadRuntime, ThreadRecord};
    /// let mut runtime = ThreadRuntime::new();
    /// runtime.register(ThreadRecord::new(123, 0));
    /// assert!(runtime.mark_finished_by_java_ref(123));
    /// ```
    #[must_use]
    pub fn mark_finished_by_java_ref(&mut self, java_ref: u64) -> bool {
        let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.java_ref == java_ref)
        else {
            return false;
        };
        record.mark_finished()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mark_finished_returns_false_for_unknown_thread_id() {
        let mut runtime = ThreadRuntime::new();
        assert!(!runtime.mark_finished(999));
    }

    use super::*;

    #[test]
    fn live_workers_counts_only_unfinished_records() {
        let mut runtime = ThreadRuntime::new();
        let finished = ThreadRecord::new(11, 1);
        let live = ThreadRecord::new(22, 2);

        let mut finished = finished;
        assert!(finished.mark_finished());
        runtime.register(finished);
        runtime.register(live);

        assert_eq!(runtime.live_workers(), 1);
        assert!(runtime.mark_finished(2));
        assert_eq!(runtime.live_workers(), 0);
        assert!(!runtime.mark_finished(2));
    }

    #[test]
    fn allocate_thread_id_is_monotonic() {
        let mut runtime = ThreadRuntime::new();
        assert_eq!(runtime.allocate_thread_id(), 0);
        assert_eq!(runtime.allocate_thread_id(), 1);
        assert_eq!(runtime.next_thread_id(), 2);
    }

    #[test]
    fn mark_finished_by_java_ref_updates_matching_record() {
        let mut runtime = ThreadRuntime::new();
        runtime.register(ThreadRecord::new(77, 9));

        assert!(runtime.mark_finished_by_java_ref(77));
        assert_eq!(runtime.live_workers(), 0);
        assert!(!runtime.mark_finished_by_java_ref(77));
        assert!(!runtime.mark_finished_by_java_ref(999));
    }
}

#[cfg(test)]
mod havoc_proptest {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_allocate_thread_id_overflow(start in i32::MAX - 10..=i32::MAX) {
            let mut runtime = ThreadRuntime { next_thread_id: start, records: vec![] };
            let _ = runtime.allocate_thread_id();
            let _ = runtime.allocate_thread_id();
        }
    }
}
