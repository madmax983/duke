//! Fuzzing utilities and hooks.
#[cfg(test)]
mod tests {
    use crate::ZipLoader;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn does_not_crash_zip_loader(data in any::<Vec<u8>>()) {
            use std::io::Write;
            let mut temp = tempfile::NamedTempFile::new().unwrap();
            temp.write_all(&data).unwrap();
            let _ = ZipLoader::open(temp.path());
        }
    }
}
