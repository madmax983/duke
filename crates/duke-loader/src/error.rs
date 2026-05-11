//! `duke-loader::error` — Errors encountered during class loading.

use thiserror::Error;

/// Errors that can occur when locating or reading `.class` files.
#[derive(Debug, Error)]
pub enum Error {
    /// The class file could not be found in the current search path.
    ///
    /// For example, `java/lang/Object` does not exist in any loader.
    #[error("class not found: {name}")]
    NotFound {
        /// The name of the missing class (e.g. `java/lang/Object`).
        name: String,
    },

    /// The requested class or resource name exceeds the maximum length.
    #[error("name too long")]
    NameTooLong,

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

/// Convenience alias for `Result<T, Error>`.
pub type Result<T> = std::result::Result<T, Error>;
