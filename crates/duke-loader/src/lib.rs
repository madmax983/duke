#![deny(missing_docs)]
//! `duke-loader` — Class file loaders and container formats.
//!
//! # Modules
//!
//! - `bootstrap` — The system bootstrap classloader.
//! - `directory` — Loads classes from standard directories (e.g. `tests/fixtures`).
//! - `error` — Errors encountered during class loading.
//! - `jimage` — Reads JDK `modules` files (`JImage` format).
//! - `manifest` — MANIFEST.MF parser for JAR files.
//! - `zip` — Read-only ZIP/JAR archive support.

use std::path::{Path, PathBuf};

pub(crate) mod bootstrap;
pub(crate) mod directory;
pub(crate) mod error;
pub(crate) mod jimage;
pub(crate) mod manifest;
pub(crate) mod zip;

pub use bootstrap::{BootstrapLoader, ClasspathEntry};
pub use directory::DirectoryLoader;
pub use error::{Error, Result};
pub use jimage::{JImageReader, ResourceInfo};
pub use manifest::parse_main_class;
pub use zip::{ZipEntryInfo, ZipLoader, ZipReader};

/// Resolved classpath resource bytes plus a stable synthetic URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedResource {
    /// The resource payload.
    pub bytes: Vec<u8>,
    /// Synthetic `file:` or `jar:file:` URL representing the resource location.
    pub url: String,
}

/// ⚡ Bolt: Eliminates intermediate String allocation and `format!` macro overhead
/// by pre-computing string capacity and using `.push_str()` sequentially.
pub(crate) fn path_to_file_url(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| PathBuf::from(path));
    let mut normalized = canonical.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        if let Some(stripped) = normalized.strip_prefix("//?/UNC/") {
            let mut s = String::with_capacity(2 + stripped.len());
            s.push_str("//");
            s.push_str(stripped);
            normalized = s;
        } else if let Some(stripped) = normalized.strip_prefix("//?/") {
            normalized = stripped.to_string();
        }
    }
    if cfg!(windows) && !normalized.starts_with('/') {
        normalized.insert(0, '/');
    }

    let mut url = String::with_capacity(7 + normalized.len());
    url.push_str("file://");
    url.push_str(&normalized);
    url
}

/// Abstraction over class file loading sources.
///
/// `name` is internal form: `"java/lang/Object"` (no `.class` suffix).
pub trait ClassLoader {
    /// Load the raw `.class` bytes for a class.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] if the class cannot be found, or
    /// another [`Error`] variant on I/O or format errors.
    fn find_class(&self, name: &str) -> Result<Vec<u8>>;

    /// Load a non-class resource by its classpath-relative name.
    ///
    /// Resource names use `/` separators and must not start with `/`; for
    /// example, `META-INF/services/java.sql.Driver`. Implementations search
    /// the same backing entry as class loading, but without appending
    /// `.class`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] if the resource is not present, or another
    /// [`Error`] when the backing archive or filesystem entry cannot be read.
    fn find_resource(&self, name: &str) -> Result<Vec<u8>> {
        Err(Error::NotFound {
            name: name.to_string(),
        })
    }

    /// Load a non-class resource together with a stable synthetic URL.
    ///
    /// By default loaders report the resource as not found until they opt into
    /// the richer metadata surface.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] if the resource is not present, or another
    /// [`Error`] when the backing archive or filesystem entry cannot be read.
    fn find_resource_entry(&self, name: &str) -> Result<LocatedResource> {
        Err(Error::NotFound {
            name: name.to_string(),
        })
    }

    /// Return every matching resource in deterministic classpath order.
    ///
    /// Classpath scanners such as `java.util.ServiceLoader` need all
    /// `META-INF/services/<binary-name>` files, not just the first hit. Simple
    /// loaders return zero or one entry; aggregate loaders concatenate child
    /// results in their search order.
    ///
    /// # Errors
    ///
    /// Returns an error if a matching resource exists but cannot be read.
    fn find_resources(&self, name: &str) -> Result<Vec<Vec<u8>>> {
        match self.find_resource(name) {
            Ok(bytes) => Ok(vec![bytes]),
            Err(Error::NotFound { .. }) => Ok(Vec::new()),
            Err(err) => Err(err),
        }
    }

    /// Return every matching resource with stable synthetic URLs.
    ///
    /// # Errors
    ///
    /// Returns an error if a matching resource exists but cannot be read.
    fn find_resource_entries(&self, name: &str) -> Result<Vec<LocatedResource>> {
        match self.find_resource_entry(name) {
            Ok(resource) => Ok(vec![resource]),
            Err(Error::NotFound { .. }) => Ok(Vec::new()),
            Err(err) => Err(err),
        }
    }

    /// Return service-provider configuration files for `service_binary_name`.
    ///
    /// This is the public primitive embedders can reuse for SPI-style
    /// discovery. It reads all resources named
    /// `META-INF/services/<service_binary_name>` across the loader in
    /// deterministic classpath order, preserving each file's bytes and leaving
    /// UTF-8/comment parsing to the caller.
    ///
    /// # Errors
    ///
    /// Returns an error if a matching service file cannot be read.
    fn service_configuration_files(&self, service_binary_name: &str) -> Result<Vec<Vec<u8>>> {
        let path = format!("META-INF/services/{service_binary_name}");
        self.find_resources(&path)
    }
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
        assert!(matches!(err, Error::NotFound { .. }));
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
        assert!(
            bytes.len() > 1000,
            "Object.class should be a reasonable size"
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

        assert!(
            cf.major_version >= 52,
            "JDK 8 uses class version 52, found {}",
            cf.major_version
        );
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
