#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(name) = std::str::from_utf8(data) {
        let loader = duke_loader::DirectoryLoader::new(std::path::PathBuf::from("/tmp/fuzz_dir"));
        let _ = duke_loader::ClassLoader::find_class(&loader, name);
        let _ = duke_loader::ClassLoader::find_resource(&loader, name);
    }
});
