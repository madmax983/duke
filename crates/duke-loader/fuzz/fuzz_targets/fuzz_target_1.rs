#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 28 {
        return;
    }
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("fuzz_jimage_{}.jimage", std::process::id()));
    if std::fs::write(&file_path, data).is_ok() {
        let _ = duke_loader::JImageReader::open(&file_path);
        let _ = std::fs::remove_file(&file_path);
    }
});
