#![allow(missing_docs)]
use duke_loader::JImageReader;
use proptest::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

proptest! {
    #[test]
    fn fuzz_jimage_open(data in proptest::collection::vec(any::<u8>(), 0..1024)) {
        let count = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("havoc_jimage_fuzz_{count}.jimage"));
        std::fs::write(&path, &data).unwrap();
        let _ = JImageReader::open(&path);
        std::fs::remove_file(&path).unwrap();
    }
}
