use std::path::Path;

use crate::{ClassLoader, DirectoryLoader, JImageReader, LoadError, LoadResult, ZipLoader};

/// A single classpath entry — either a directory or a ZIP/JAR archive.
pub enum ClasspathEntry {
    /// Loads `.class` files from a filesystem directory.
    Directory(DirectoryLoader),
    /// Loads `.class` files from a ZIP or JAR archive.
    Zip(ZipLoader),
}

impl ClassLoader for ClasspathEntry {
    fn find_class(&self, name: &str) -> LoadResult<Vec<u8>> {
        match self {
            Self::Directory(d) => d.find_class(name),
            Self::Zip(z) => z.find_class(name),
        }
    }
}

/// Bootstrap class loader.
///
/// Resolves classes by trying the JDK jimage first (for standard library
/// classes), then falling through to classpath entries (directories or JARs)
/// for application classes.
pub struct BootstrapLoader {
    jimage: JImageReader,
    classpath: Vec<ClasspathEntry>,
}

impl BootstrapLoader {
    /// Create a new bootstrap loader.
    ///
    /// - `modules_path`: path to the JDK `lib/modules` jimage file.
    /// - `classpath_paths`: directories or JAR/ZIP files to search for
    ///   application classes.  Paths ending in `.jar` or `.zip` are opened
    ///   as archives; everything else is treated as a directory.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError`] if the jimage file or any JAR cannot be opened.
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
    pub fn new(modules_path: &Path, classpath_paths: Vec<impl AsRef<Path>>) -> LoadResult<Self> {
        let jimage = JImageReader::open(modules_path)?;
        let mut classpath = Vec::new();
        for p in classpath_paths {
            classpath.push(classpath_entry_for(p.as_ref())?);
        }
        Ok(Self { jimage, classpath })
    }
}

impl ClassLoader for BootstrapLoader {
    fn find_class(&self, name: &str) -> LoadResult<Vec<u8>> {
        // Standard library: try jimage first
        if let Ok(bytes) = self.jimage.find_class(name) {
            return Ok(bytes);
        }
        // Application classes: classpath entries (directories and JARs)
        for entry in &self.classpath {
            if let Ok(bytes) = entry.find_class(name) {
                return Ok(bytes);
            }
        }
        Err(LoadError::NotFound {
            name: name.to_string(),
        })
    }
}

/// Auto-detect whether a path is a JAR/ZIP or a directory and build the
/// appropriate classpath entry.
fn classpath_entry_for(path: &Path) -> LoadResult<ClasspathEntry> {
    let is_archive = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("jar") || ext.eq_ignore_ascii_case("zip"));
    if is_archive {
        Ok(ClasspathEntry::Zip(ZipLoader::open(path)?))
    } else {
        Ok(ClasspathEntry::Directory(DirectoryLoader::new(path)))
    }
}
