use duke_loader::ZipLoader;
use proptest::prelude::*;
use std::path::PathBuf;

proptest! {
    #[test]
    fn test_zip_loader_oob(data in any::<Vec<u8>>()) {

        let p = PathBuf::from("temp_fuzz_zip.zip");
        std::fs::write(&p, &data).unwrap();
        let _ = ZipLoader::open(&p);
        let _ = std::fs::remove_file(&p);
    }
}
