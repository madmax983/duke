//! `duke-loader` — Class file loaders and container formats.
//!
//! # Modules
//!
//! - [`bootstrap`] — The system bootstrap classloader.
//! - [`directory`] — Loads classes from standard directories (e.g. `tests/fixtures`).
//! - [`error`] — Errors encountered during class loading.
//! - [`jimage`] — Reads JDK `modules` files (`JImage` format).
//! - [`manifest`] — MANIFEST.MF parser for JAR files.
//! - [`zip`] — Read-only ZIP/JAR archive support.

pub mod bootstrap;
pub mod directory;
pub mod error;
pub mod jimage;
pub mod manifest;
pub mod zip;

pub use bootstrap::{BootstrapLoader, ClasspathEntry};
pub use directory::DirectoryLoader;
pub use error::{Error, LoadError, LoadResult, Result};
pub use jimage::JImageReader;
pub use manifest::parse_main_class;
pub use zip::{ZipEntryInfo, ZipLoader, ZipReader};

/// Abstraction over class file loading sources.
///
/// `name` is internal form: `"java/lang/Object"` (no `.class` suffix).
pub trait ClassLoader {
    /// Load the raw `.class` bytes for a class.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError::NotFound`] if the class cannot be found, or
    /// another [`LoadError`] variant on I/O or format errors.
    fn find_class(&self, name: &str) -> LoadResult<Vec<u8>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jdk_modules_path() -> std::path::PathBuf {
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            let path = std::path::PathBuf::from(java_home).join("lib/modules");
            if path.exists() {
                return path;
            }
        }
        std::path::PathBuf::from("/usr/lib/jvm/java-21-openjdk-amd64/lib/modules")
    }

    // -----------------------------------------------------------------------
    // Trait sanity
    // -----------------------------------------------------------------------

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Box<dyn ClassLoader>> = None;
    }

    // -----------------------------------------------------------------------
    // DirectoryLoader
    // -----------------------------------------------------------------------

    #[test]
    fn directory_loader_finds_hello_world() {
        let fixtures =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
        let loader = DirectoryLoader::new(fixtures);
        let bytes = loader
            .find_class("HelloWorld")
            .expect("should find HelloWorld");
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
    }

    #[test]
    fn directory_loader_returns_not_found() {
        let loader = DirectoryLoader::new("/nonexistent/path");
        let err = loader.find_class("NoSuchClass").unwrap_err();
        assert!(matches!(err, LoadError::NotFound { .. }));
    }

    // -----------------------------------------------------------------------
    // JImageReader
    // -----------------------------------------------------------------------

    #[test]
    fn jimage_reader_opens_modules_file() {
        let path = jdk_modules_path();
        if !path.exists() {
            eprintln!("skipping — JDK modules not found at {path:?}");
            return;
        }
        let reader = JImageReader::open(&path).expect("should open jimage");
        assert!(reader.resource_count() > 0, "should have resources");
    }

    #[test]
    fn jimage_can_find_object_class_location() {
        let path = jdk_modules_path();
        if !path.exists() {
            return;
        }
        let reader = JImageReader::open(&path).expect("open");
        let info = reader
            .find_resource("/java.base/java/lang/Object.class")
            .expect("Object.class must exist in java.base");
        assert!(info.uncompressed > 0);
    }

    #[test]
    fn jimage_reads_object_class_bytes() {
        let path = jdk_modules_path();
        if !path.exists() {
            return;
        }
        let reader = JImageReader::open(&path).expect("open");
        let bytes = reader
            .read_resource("/java.base/java/lang/Object.class")
            .expect("read Object.class");
        assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE], "bad magic");
        assert_eq!(
            bytes.len(),
            2487,
            "Object.class should be 2487 bytes (JDK 21.0.4)"
        );
    }

    // -----------------------------------------------------------------------
    // BootstrapLoader
    // -----------------------------------------------------------------------

    #[test]
    fn bootstrap_loader_loads_from_jimage_and_classpath() {
        let jdk_modules = jdk_modules_path();
        if !jdk_modules.exists() {
            return;
        }
        let fixtures =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");

        let loader =
            BootstrapLoader::new(&jdk_modules, vec![fixtures]).expect("create bootstrap loader");

        let obj = loader
            .find_class("java/lang/Object")
            .expect("Object from jimage");
        assert_eq!(&obj[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);

        let hw = loader
            .find_class("HelloWorld")
            .expect("HelloWorld from classpath");
        assert_eq!(&hw[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
    }

    // -----------------------------------------------------------------------
    // Integration: parse loaded classes with duke-classfile
    // -----------------------------------------------------------------------

    #[test]
    fn loaded_object_class_parses_correctly() {
        let jdk_modules = jdk_modules_path();
        if !jdk_modules.exists() {
            return;
        }
        let loader = BootstrapLoader::new(&jdk_modules, vec![] as Vec<std::path::PathBuf>)
            .expect("create loader");
        let bytes = loader.find_class("java/lang/Object").expect("load Object");
        let cf = duke_classfile::parse(&bytes).expect("parse Object.class");

        assert_eq!(cf.major_version, 65, "JDK 21 uses class version 65");
        assert_eq!(cf.super_class.0, 0, "java.lang.Object has no super");
    }

    #[test]
    fn loaded_hello_world_parses_correctly() {
        let fixtures =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
        let loader = DirectoryLoader::new(fixtures);
        let bytes = loader.find_class("HelloWorld").expect("load HelloWorld");
        let cf = duke_classfile::parse(&bytes).expect("parse HelloWorld.class");
        assert_eq!(cf.major_version, 65);
    }
}

#[cfg(test)]
mod proptest_open;
