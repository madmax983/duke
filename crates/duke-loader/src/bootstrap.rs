//! `duke-loader::bootstrap` — The system bootstrap classloader.

use std::path::Path;

use crate::{ClassLoader, DirectoryLoader, JImageReader, LoadError, LoadResult, ZipLoader};

/// A single classpath entry — either a directory or a ZIP/JAR archive.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use duke_loader::{DirectoryLoader, ClasspathEntry};
///
/// let dir_loader = DirectoryLoader::new(PathBuf::from("my_classes"));
/// let entry = ClasspathEntry::Directory(dir_loader);
/// ```
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
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use duke_loader::{BootstrapLoader, ClassLoader};
///
/// let loader = BootstrapLoader::new(&PathBuf::from("lib/modules"), vec!["app.jar"]).unwrap();
/// let bytes = loader.find_class("java/lang/Object").unwrap();
/// ```
pub struct BootstrapLoader {
    jimage: JImageReader,
    classpath: Vec<ClasspathEntry>,
}

impl BootstrapLoader {
    #[cfg(test)]
    pub(crate) fn new_for_test(classpath: Vec<ClasspathEntry>) -> Self {
        Self {
            jimage: JImageReader::empty_for_test(),
            classpath,
        }
    }

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
        let mut classpath = Vec::with_capacity(classpath_paths.len());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    /// Creates a minimal valid ZIP file for testing.
    fn create_dummy_zip(path: &Path) {
        let mut file = fs::File::create(path).unwrap();
        // A minimal valid ZIP needs at least an End of Central Directory record.
        // EOCD signature is 0x06054b50
        let eocd = [
            0x50, 0x4b, 0x05, 0x06, // signature
            0x00, 0x00, // disk number
            0x00, 0x00, // disk with CD
            0x00, 0x00, // entries on disk
            0x00, 0x00, // total entries
            0x00, 0x00, 0x00, 0x00, // CD size
            0x00, 0x00, 0x00, 0x00, // CD offset
            0x00, 0x00, // comment length
        ];
        file.write_all(&eocd).unwrap();
    }

    #[test]
    fn test_classpath_entry_for_directory() {
        let root = std::env::temp_dir().join("duke_test_dir_1");
        fs::create_dir_all(&root).unwrap();

        let entry = classpath_entry_for(&root).expect("Failed to create classpath entry");
        assert!(matches!(entry, ClasspathEntry::Directory(_)));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_classpath_entry_find_class_zip_delegation() {
        let root = std::env::temp_dir().join("duke_test_dir_zip_delegation");
        fs::create_dir_all(&root).unwrap();
        let zip_path = root.join("test.zip");
        create_dummy_zip(&zip_path);

        let entry = classpath_entry_for(&zip_path).unwrap();

        // Request a missing class (since dummy zip is empty)
        let err = entry
            .find_class("java/lang/Missing")
            .expect_err("Class should not be found in empty zip");
        assert!(matches!(err, LoadError::NotFound { .. }));

        // Drop the entry to release the file handle before attempting to delete the directory
        drop(entry);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_classpath_entry_for_zip() {
        let root = std::env::temp_dir().join("duke_test_dir_2");
        fs::create_dir_all(&root).unwrap();
        let zip_path = root.join("test.zip");
        create_dummy_zip(&zip_path);

        let entry = classpath_entry_for(&zip_path).expect("Failed to create zip classpath entry");
        assert!(matches!(entry, ClasspathEntry::Zip(_)));

        drop(entry);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_classpath_entry_find_class_delegation() {
        let root = std::env::temp_dir().join("duke_test_dir_3");
        // Setup a directory classpath entry with a fake class file
        let class_dir = root.join("java").join("lang");
        fs::create_dir_all(&class_dir).unwrap();
        fs::write(class_dir.join("Object.class"), b"dummy class bytes").unwrap();

        let entry = classpath_entry_for(&root).unwrap();

        // Find the class we just wrote
        let bytes = entry
            .find_class("java/lang/Object")
            .expect("Class should be found");
        assert_eq!(bytes, b"dummy class bytes");

        // Request a missing class
        let err = entry
            .find_class("java/lang/Missing")
            .expect_err("Class should not be found");
        assert!(matches!(err, LoadError::NotFound { .. }));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_bootstrap_loader_new_invalid_jimage() {
        // Provide an invalid jimage path
        let result = BootstrapLoader::new(Path::new("/does/not/exist"), Vec::<&Path>::new());
        match result {
            Err(LoadError::Io { .. }) => {}
            _ => panic!("Expected LoadError::Io, got something else"),
        }
    }

    #[test]
    fn test_bootstrap_loader_find_class_fallback() {
        let root = std::env::temp_dir().join("duke_test_dir_4");
        let class_dir = root.join("java").join("lang");
        fs::create_dir_all(&class_dir).unwrap();
        fs::write(class_dir.join("Object.class"), b"app class bytes").unwrap();

        let cp_entry = classpath_entry_for(&root).unwrap();

        // Construct loader with an empty JImage, so it must fallback to the classpath
        let loader = BootstrapLoader::new_for_test(vec![cp_entry]);

        let bytes = loader
            .find_class("java/lang/Object")
            .expect("Class should be found in classpath");
        assert_eq!(bytes, b"app class bytes");

        let err = loader
            .find_class("java/lang/Missing")
            .expect_err("Should not find missing class");
        assert!(matches!(err, LoadError::NotFound { .. }));

        fs::remove_dir_all(&root).unwrap();
    }
}
