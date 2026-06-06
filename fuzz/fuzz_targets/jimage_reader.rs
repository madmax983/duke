#![no_main]

use libfuzzer_sys::fuzz_target;
use duke_loader::{JImageReader, ClassLoader};
use std::io::Write;
use tempfile::NamedTempFile;

fuzz_target!(|data: &[u8]| {
    if let Ok(mut file) = NamedTempFile::new() {
        if file.write_all(data).is_ok() {
            if let Ok(reader) = JImageReader::open(file.path()) {
                let _ = reader.read_resource("test");
            }
        }
    }
});
