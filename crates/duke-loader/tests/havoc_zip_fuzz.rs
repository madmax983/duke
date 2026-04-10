use duke_loader::ZipLoader;
use proptest::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

proptest! {
    #[test]
    fn fuzz_zip_loader_open(data in any::<Vec<u8>>()) {
        let zip_path = std::env::temp_dir().join(format!("havoc_zip_fuzz_{}.zip", COUNTER.fetch_add(1, Ordering::SeqCst)));
        std::fs::write(&zip_path, &data).unwrap();
        let _ = ZipLoader::open(&zip_path);
        std::fs::remove_file(&zip_path).unwrap();
    }
}
