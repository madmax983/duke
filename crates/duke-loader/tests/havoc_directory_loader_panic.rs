#![allow(missing_docs)]

use duke_loader::{ClassLoader, DirectoryLoader, LoadError};

#[test]
fn directory_loader_returns_not_found_on_empty_component() {
    let loader = DirectoryLoader::new(std::path::PathBuf::from("/tmp"));
    let err = loader.find_class("java//lang/Object").unwrap_err();
    assert!(matches!(err, LoadError::NotFound { .. }));
}

#[test]
fn directory_loader_should_prevent_absolute_paths_windows() {
    let loader = DirectoryLoader::new(std::path::PathBuf::from("/tmp"));
    let result = loader.find_class("C:\\Windows\\System32\\cmd");
    assert!(
        matches!(result, Err(LoadError::NotFound { .. })),
        "Vulnerability triggered! Got {result:?}"
    );
}

#[test]
fn directory_loader_should_prevent_directory_traversal() {
    let root = std::env::temp_dir().join("duke_loader_tests");
    std::fs::create_dir_all(&root).unwrap();

    let secret_file = root.parent().unwrap().join("secret.class");
    std::fs::write(&secret_file, "SENSITIVE_DATA").unwrap();

    let loader = DirectoryLoader::new(&root);

    // Exploit: try to read outside root.
    let result = loader.find_class("../secret");

    std::fs::remove_file(&secret_file).unwrap();
    std::fs::remove_dir_all(&root).unwrap();

    assert!(
        matches!(result, Err(LoadError::NotFound { .. })),
        "Vulnerability triggered! Got {result:?}"
    );
}
