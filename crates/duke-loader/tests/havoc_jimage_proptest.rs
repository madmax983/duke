use proptest::prelude::*;
use std::path::PathBuf;

proptest! {
    #[test]
    fn test_jimage_oob(data in any::<Vec<u8>>()) {

        let p = PathBuf::from("temp_fuzz_jimage.jimage");
        std::fs::write(&p, &data).unwrap();
        let _ = duke_loader::JImageReader::open(&p);
        let _ = std::fs::remove_file(&p);
    }
}
