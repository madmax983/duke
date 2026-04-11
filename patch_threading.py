import re

with open("crates/duke-interpreter/src/threading.rs", "r") as f:
    content = f.read()

content_new = content.replace(
"""pub struct ThreadRecord {
    pub java_ref: u64,
    pub thread_id: i32,
    pub finished: bool,
    pub daemon: bool,
}

impl ThreadRecord {
    #[must_use]
    pub const fn new(java_ref: u64, thread_id: i32) -> Self {
        Self {
            java_ref,
            thread_id,
            finished: false,
            daemon: false,
        }
    }""",
"""pub struct ThreadRecord {
    pub java_ref: u64,
    pub thread_id: i32,
    pub finished: bool,
    pub daemon: bool,
    pub rust_thread_id: Option<std::thread::ThreadId>,
}

impl ThreadRecord {
    #[must_use]
    pub const fn new(java_ref: u64, thread_id: i32) -> Self {
        Self {
            java_ref,
            thread_id,
            finished: false,
            daemon: false,
            rust_thread_id: None,
        }
    }""")

content_new = content_new.replace(
"""    pub fn mark_finished_by_java_ref(&mut self, java_ref: u64) -> bool {
        let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.java_ref == java_ref)
        else {
            return false;
        };
        record.mark_finished()
    }""",
"""    pub fn mark_finished_by_java_ref(&mut self, java_ref: u64) -> bool {
        let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.java_ref == java_ref)
        else {
            return false;
        };
        record.mark_finished()
    }

    pub fn set_rust_thread_id(&mut self, java_ref: u64, id: std::thread::ThreadId) {
        if let Some(record) = self.records.iter_mut().find(|r| r.java_ref == java_ref) {
            record.rust_thread_id = Some(id);
        }
    }""")

with open("crates/duke-interpreter/src/threading.rs", "w") as f:
    f.write(content_new)
