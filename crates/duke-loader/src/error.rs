use thiserror::Error;

/// Errors that can occur when locating or reading `.class` files.
#[derive(Debug, Error)]
pub enum LoadError {
    /// The class file could not be found in the current search path.
    ///
    /// For example, `java/lang/Object` does not exist in any loader.
    #[error("class not found: {name}")]
    NotFound { name: String },

    /// An I/O error occurred reading the `.class` file or container.
    #[error("I/O error reading '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// The JImage (`modules`) container format is invalid or corrupted.
    #[error("jimage format error: {msg}")]
    JImageFormat { msg: String },

    /// A resource inside the JImage container failed to decompress.
    #[error("jimage decompression error for '{name}'")]
    Decompress { name: String },
}

/// Convenience alias for `Result<T, LoadError>`.
pub type LoadResult<T> = Result<T, LoadError>;
