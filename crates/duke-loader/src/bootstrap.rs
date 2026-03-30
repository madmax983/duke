//! The `bootstrap` module provides the system bootstrap classloader.
//!
//! This module contains the [`BootstrapLoader`] which is responsible for resolving
//! classes by first trying the JDK `jimage` (for standard library classes) and
//! then falling back to classpath directories (for application classes).

use std::path::Path;

use crate::{ClassLoader, DirectoryLoader, JImageReader, LoadError, LoadResult};

/// Bootstrap class loader.
///
/// Resolves classes by trying the JDK jimage first (for standard library
/// classes), then falling through to classpath directories (for application
/// classes).
pub struct BootstrapLoader {
    jimage: JImageReader,
    classpath: Vec<DirectoryLoader>,
}

impl BootstrapLoader {
    /// Create a new bootstrap loader.
    ///
    /// - `modules_path`: path to the JDK `lib/modules` jimage file.
    /// - `classpath_dirs`: directories to search for application classes.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError`] if the jimage file cannot be opened.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_loader::{ClassLoader, BootstrapLoader};
    /// use std::path::PathBuf;
    ///
    /// // In practice, `modules_path` points to a real JDK 21 `lib/modules` file.
    /// // If the file is missing or invalid, it returns a LoadError.
    /// let result = BootstrapLoader::new(
    ///     &PathBuf::from("/invalid/path/to/lib/modules"),
    ///     vec!["my_classes", "other_classes"]
    /// );
    ///
    /// assert!(result.is_err());
    /// ```
    pub fn new(modules_path: &Path, classpath_dirs: Vec<impl AsRef<Path>>) -> LoadResult<Self> {
        let jimage = JImageReader::open(modules_path)?;
        let classpath = classpath_dirs
            .into_iter()
            .map(|p| DirectoryLoader::new(p))
            .collect();
        Ok(Self { jimage, classpath })
    }
}

impl ClassLoader for BootstrapLoader {
    fn find_class(&self, name: &str) -> LoadResult<Vec<u8>> {
        // Standard library: try jimage first
        if let Ok(bytes) = self.jimage.find_class(name) {
            return Ok(bytes);
        }
        // Application classes: classpath directories
        for loader in &self.classpath {
            if let Ok(bytes) = loader.find_class(name) {
                return Ok(bytes);
            }
        }
        Err(LoadError::NotFound {
            name: name.to_string(),
        })
    }
}
