//! `duke-loader::proptest_open` — Fuzzing utilities for class loaders

use super::jimage::JImageReader;
use proptest::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

proptest! {
    #[test]
    fn fuzz_jimage_open_inline(bytes in proptest::collection::vec(any::<u8>(), 0..1024)) {
        let temp_dir = std::env::temp_dir();
        let c = COUNTER.fetch_add(1, Ordering::SeqCst);
        let file_path = temp_dir.join(format!("fuzz_jimage_open_inline_{c}.jimage"));
        std::fs::write(&file_path, &bytes).unwrap();
        let _ = JImageReader::open(&file_path);
        let _ = std::fs::remove_file(&file_path);
    }
}
