#![no_main]

use libfuzzer_sys::fuzz_target;
use duke_loader::ClassLoader;

fuzz_target!(|data: &[u8]| {
    if data.len() < 22 {
        return;
    }
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("fuzz_zip_{}.zip", std::process::id()));
    if std::fs::write(&file_path, data).is_ok() {
        if let Ok(loader) = duke_loader::ZipLoader::open(&file_path) {
            let _ = loader.find_class("Test");
            for name in loader.reader().entry_names() {
                let _ = loader.reader().read_entry(name);
            }
        }
        let _ = std::fs::remove_file(&file_path);
    }
});