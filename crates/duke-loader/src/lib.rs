//! `duke-loader` — Class file loaders and container formats.
//!
//! Welcome to the library of Alexandria for Duke JVM! This crate handles locating and reading
//! raw `.class` files and auxiliary resources from various storage backends. It provides a unified
//! interface (`ClassLoader`) that abstracts away the underlying file system, archives, or JVM-specific formats.
//!
//! ## The `ClassLoader` Trait
//! The `ClassLoader` trait is the core API of this crate. The JVM calls into it when it needs to
//! resolve a class name (e.g., `java/lang/Object`) into its raw bytecode representation.
//!
//! ## Storage Backends
//! We provide multiple implementations of `ClassLoader` to support the diverse ways Java code is packaged:
//! - **`DirectoryLoader`**: Reads files directly from the filesystem. Excellent for development or executing
//!   locally compiled `.class` files.
//! - **`ZipLoader`**: Reads files directly from `.jar` or `.zip` archives. It uses a memory-mapped, lazy-loading
//!   strategy to keep memory overhead incredibly low while avoiding full extraction.
//! - **`JImageReader`**: Support for the modern Java 9+ module system (`lib/modules`). Parses the highly-optimized
//!   custom container format introduced to replace `rt.jar`.
//! - **`BootstrapLoader`**: The orchestrator. It sits at the top of the hierarchy, first attempting to load
//!   core JDK classes via `JImageReader`, and then falling back to user-provided `DirectoryLoader` or `ZipLoader`
//!   instances for application code.
//!
//! ## Robustness
//! Loaders must never panic on missing files or malformed archives. The `Result` type is strictly enforced, ensuring
//! the JVM can gracefully throw a `ClassNotFoundException` rather than crashing the host process.
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
///
/// # Examples
///
/// ```
/// use duke_loader::{ClassLoader, DirectoryLoader};
/// use std::path::PathBuf;
///
/// let loader = DirectoryLoader::new(PathBuf::from("../../tests/fixtures"));
/// // 1. Find a class
/// let class_bytes = loader.find_class("HelloWorld").unwrap();
/// assert_eq!(&class_bytes[0..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
///
/// // 2. Find a non-class resource (The hello_world.txt is in our fixtures)
/// let resource_bytes = loader.find_resource("hello_world.txt").unwrap();
/// assert!(resource_bytes.starts_with(b"Hello World"));
/// ```
pub trait ClassLoader {
    /// Load the raw `.class` bytes for a class.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_loader::{ClassLoader, DirectoryLoader};
    /// use std::path::PathBuf;
    ///
    /// let loader = DirectoryLoader::new(PathBuf::from("../../tests/fixtures"));
    /// let bytes = loader.find_class("HelloWorld").unwrap();
    /// assert_eq!(&bytes[0..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
    /// ```
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
    /// # Examples
    ///
    /// ```
    /// use duke_loader::{ClassLoader, DirectoryLoader};
    /// use std::path::PathBuf;
    ///
    /// let loader = DirectoryLoader::new(PathBuf::from("../../tests/fixtures"));
    /// let bytes = loader.find_resource("hello_world.txt").unwrap();
    /// assert!(bytes.starts_with(b"Hello World"));
    /// ```
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
    /// # Examples
    ///
    /// ```
    /// use duke_loader::{ClassLoader, DirectoryLoader};
    /// use std::path::PathBuf;
    ///
    /// let loader = DirectoryLoader::new(PathBuf::from("../../tests/fixtures"));
    /// let resource_res = loader.find_resource_entry("hello_world.txt").unwrap();
    /// assert!(resource_res.url.starts_with("file://"));
    /// ```
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
    /// # Examples
    ///
    /// ```
    /// use duke_loader::{ClassLoader, DirectoryLoader};
    /// use std::path::PathBuf;
    ///
    /// let loader = DirectoryLoader::new(PathBuf::from("../../tests/fixtures"));
    /// let resources = loader.find_resources("hello_world.txt").unwrap();
    /// assert_eq!(resources.len(), 1);
    /// ```
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
    /// # Examples
    ///
    /// ```
    /// use duke_loader::{ClassLoader, DirectoryLoader};
    /// use std::path::PathBuf;
    ///
    /// let loader = DirectoryLoader::new(PathBuf::from("../../tests/fixtures"));
    /// let resources = loader.find_resource_entries("hello_world.txt").unwrap();
    /// assert_eq!(resources.len(), 1);
    /// ```
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
    /// # Examples
    ///
    /// ```
    /// use duke_loader::{ClassLoader, DirectoryLoader};
    /// use std::path::PathBuf;
    ///
    /// let loader = DirectoryLoader::new(PathBuf::from("../../tests/fixtures"));
    /// // In a real project, this would read all META-INF/services/java.sql.Driver files
    /// let configs = loader.service_configuration_files("java.sql.Driver").unwrap();
    /// assert_eq!(configs.len(), 0);
    /// ```
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
