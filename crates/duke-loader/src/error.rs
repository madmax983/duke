use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("class not found: {name}")]
    NotFound { name: String },

    #[error("I/O error reading '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("jimage format error: {msg}")]
    JImageFormat { msg: String },

    #[error("jimage decompression error for '{name}'")]
    Decompress { name: String },
}

pub type LoadResult<T> = Result<T, LoadError>;
