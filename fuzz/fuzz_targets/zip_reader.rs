#![no_main]

use libfuzzer_sys::fuzz_target;
use duke_loader::ZipReader;

fuzz_target!(|data: &[u8]| {
    if let Ok(reader) = ZipReader::from_bytes(data.to_vec()) {
        let _ = reader.read_entry("test");
        let _ = reader.read_entry("test.txt");
        let _ = reader.entry_names().count();
    }
});
