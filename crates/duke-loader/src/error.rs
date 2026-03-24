//! `duke-loader::error` — Errors encountered during class loading.

use thiserror::Error;

/// Errors that can occur when locating or reading `.class` files.
///
/// # Examples
///
/// Demonstrating the formatting for various loading failures:
///
/// ```
/// use duke_loader::LoadError;
///
/// assert_eq!(
///     LoadError::NotFound { name: "java/lang/Object".into() }.to_string(),
///     "class not found: java/lang/Object"
/// );
///
/// assert_eq!(
///     LoadError::Io { path: "rt.jar".into(), source: std::io::Error::from(std::io::ErrorKind::NotFound) }.to_string(),
///     "I/O error reading 'rt.jar': entity not found"
/// );
///
/// assert_eq!(
///     LoadError::JImageFormat { msg: "invalid magic".into() }.to_string(),
///     "jimage format error: invalid magic"
/// );
///
/// assert_eq!(
///     LoadError::Decompress { name: "/java.base/java/lang/Object.class".into() }.to_string(),
///     "jimage decompression error for '/java.base/java/lang/Object.class'"
/// );
///
/// assert_eq!(
///     LoadError::ZipFormat { msg: "missing EOCD".into() }.to_string(),
///     "ZIP format error: missing EOCD"
/// );
///
/// assert_eq!(
///     LoadError::ZipCrc32 { name: "Main.class".into(), expected: 0xCAFEBABE, actual: 0xDEADBEEF }.to_string(),
///     "ZIP CRC32 mismatch for 'Main.class': expected 0xcafebabe, got 0xdeadbeef"
/// );
/// ```
#[derive(Debug, Error)]
pub enum LoadError {
    /// The class file could not be found in the current search path.
    ///
    /// For example, `java/lang/Object` does not exist in any loader.
    #[error("class not found: {name}")]
    NotFound {
        /// The name of the missing class (e.g. `java/lang/Object`).
        name: String,
    },

    /// An I/O error occurred reading the `.class` file or container.
    #[error("I/O error reading '{path}': {source}")]
    Io {
        /// The path that failed to read.
        path: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The `JImage` (`modules`) container format is invalid or corrupted.
    #[error("jimage format error: {msg}")]
    JImageFormat {
        /// Reason for format error.
        msg: String,
    },

    /// A resource inside the `JImage` container failed to decompress.
    #[error("jimage decompression error for '{name}'")]
    Decompress {
        /// Name of the resource that failed.
        name: String,
    },

    /// The ZIP/JAR archive format is invalid or corrupted.
    #[error("ZIP format error: {msg}")]
    ZipFormat {
        /// Reason for format error.
        msg: String,
    },

    /// A ZIP entry's CRC32 checksum does not match after decompression.
    #[error("ZIP CRC32 mismatch for '{name}': expected {expected:#010x}, got {actual:#010x}")]
    ZipCrc32 {
        /// Name of the entry that failed.
        name: String,
        /// Expected CRC32 from the central directory.
        expected: u32,
        /// Actual CRC32 computed from decompressed bytes.
        actual: u32,
    },
}

/// Convenience alias for `Result<T, LoadError>`.
///
/// # Examples
///
/// ```
/// use duke_loader::{LoadError, LoadResult};
///
/// fn find_class(exists: bool) -> LoadResult<Vec<u8>> {
///     if exists {
///         Ok(vec![0xCA, 0xFE, 0xBA, 0xBE])
///     } else {
///         Err(LoadError::NotFound { name: "HelloWorld".into() })
///     }
/// }
///
/// assert!(find_class(true).is_ok());
/// assert!(find_class(false).is_err());
/// ```
pub type LoadResult<T> = Result<T, LoadError>;
