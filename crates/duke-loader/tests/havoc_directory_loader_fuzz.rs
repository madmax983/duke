#![allow(missing_docs)]

use duke_loader::{ClassLoader, DirectoryLoader};
use proptest::prelude::*;

proptest! {
    /// DirectoryLoader must never panic and must never access files outside its root.
    #[test]
    fn fuzz_directory_loader_find_class(name in "\\w{1,10}(/\\w{1,10})*(\\.\\.|\\.|/|\\\\|C:|:\\w{1,5})?") {
        let root = std::env::temp_dir().join("duke_loader_fuzz_tests");
        let _ = std::fs::create_dir_all(&root);

        let loader = DirectoryLoader::new(&root);
        let result = loader.find_class(&name);

        let is_unsafe = name.contains("..") || name.contains('.') || name.starts_with('/') || name.starts_with('\\') || name.contains(':');

        if is_unsafe {
            assert!(result.is_err(), "Unsafe path {name:?} was not rejected");
        }

        let _ = std::fs::remove_dir_all(&root);
    }
}
