#![allow(dead_code)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Marker for a Java thread action that the runtime can pause on later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadPause {
    Sleep(Duration),
    Join { thread_id: i32 },
}

/// Shared output sink placeholder for future threaded interpreter writes.
#[derive(Debug, Clone, Default)]
pub struct SharedOutput {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl SharedOutput {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_buffer(buffer: Arc<Mutex<Vec<u8>>>) -> Self {
        Self { buffer }
    }

    #[must_use]
    pub fn buffer(&self) -> Arc<Mutex<Vec<u8>>> {
        Arc::clone(&self.buffer)
    }
}

/// Minimal metadata for a Java thread record.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThreadRecord {
    pub java_ref: u64,
    pub thread_id: i32,
    pub finished: bool,
    pub daemon: bool,
}

impl ThreadRecord {
    #[must_use]
    pub fn new(java_ref: u64, thread_id: i32) -> Self {
        Self {
            java_ref,
            thread_id,
            finished: false,
            daemon: false,
        }
    }

    /// Mark this record as finished and report whether the state changed.
    pub fn mark_finished(&mut self) -> bool {
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
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn next_thread_id(&self) -> i32 {
        self.next_thread_id
    }

    #[must_use]
    pub fn live_workers(&self) -> usize {
        self.records
            .iter()
            .filter(|record| !record.finished)
            .count()
    }

    #[must_use]
    pub fn records(&self) -> &[ThreadRecord] {
        &self.records
    }

    #[must_use]
    pub fn allocate_thread_id(&mut self) -> i32 {
        let id = self.next_thread_id;
        self.next_thread_id += 1;
        id
    }

    pub fn register(&mut self, record: ThreadRecord) {
        self.records.push(record);
    }

    /// Mark a worker as finished by thread id.
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

    /// Mark a worker as finished by Java thread reference.
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
