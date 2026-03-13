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
    /// Forges a new `BootstrapLoader` capable of loading both system and user classes.
    ///
    /// The JVM needs to know where its core libraries live (like `java.lang.Object`), as well
    /// as any application-specific logic provided by the user. This constructor wires together
    /// the specialized `lib/modules` reader and the standard filesystem loaders.
    ///
    /// # Parameters
    ///
    /// * `modules_path` - The absolute path to the `lib/modules` file from a standard OpenJDK
    ///   installation. This is where the core Java classes reside.
    /// * `classpath_dirs` - A sequence of directories where the loader should look for `.class` files
    ///   when a class cannot be found in the JDK image.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use duke_loader::BootstrapLoader;
    ///
    /// let jdk_path = Path::new("/usr/lib/jvm/java-21-openjdk/lib/modules");
    /// let user_paths = vec!["./target/classes", "./bin"];
    ///
    /// // If the JDK file doesn't exist or is corrupted, this will fail.
    /// let loader = BootstrapLoader::new(jdk_path, user_paths).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a [`LoadError::Io`] if the `jimage` file at `modules_path` cannot be
    /// opened, or if it has an invalid structure.
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
