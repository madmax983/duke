#![no_main]

use libfuzzer_sys::fuzz_target;
use duke_loader::{ClassLoader, DirectoryLoader};
use std::path::PathBuf;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let loader = DirectoryLoader::new(PathBuf::from("/tmp"));
        let _ = loader.find_class(s);
        let _ = loader.find_resource(s);
        let _ = loader.find_resource_entry(s);
        let _ = loader.find_resources(s);
        let _ = loader.find_resource_entries(s);
        let _ = loader.service_configuration_files(s);
    }
});
