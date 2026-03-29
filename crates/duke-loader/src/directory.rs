//! `duke-loader::directory` — Loads classes from standard directories (e.g. `tests/fixtures`).

use std::path::{Path, PathBuf};

use crate::{ClassLoader, LoadError, LoadResult};

/// Loads `.class` files from a filesystem directory.
///
/// Given `find_class("java/lang/Object")`, looks for
/// `{root}/java/lang/Object.class`.
pub struct DirectoryLoader {
    root: PathBuf,
}

impl DirectoryLoader {
    /// Creates a new `DirectoryLoader` rooted at the given directory.
    ///
    /// The loader will resolve internal class names (like `java/lang/Object`)
    /// by searching for `{root}/java/lang/Object.class`.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_loader::{ClassLoader, DirectoryLoader};
    /// use std::path::PathBuf;
    ///
    /// let loader = DirectoryLoader::new(PathBuf::from("my_classes"));
    ///
    /// // This will look for "my_classes/com/example/Main.class"
    /// let _result = loader.find_class("com/example/Main");
    /// ```
    #[must_use]
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }
}

impl ClassLoader for DirectoryLoader {
    fn find_class(&self, name: &str) -> LoadResult<Vec<u8>> {
        let mut path = self.root.clone();
        // name is "java/lang/Object" — split on '/' to build OS path + ".class"

        // Prevent Windows absolute paths and directory traversal
        if name.contains("..")
            || name.contains('.')
            || name.starts_with('/')
            || name.starts_with('\\')
            || name.contains(':')
        {
            return Err(LoadError::NotFound {
                name: name.to_string(),
            });
        }

        for component in name.split(['/', '\\']) {
            if component.is_empty() {
                return Err(LoadError::NotFound {
                    name: name.to_string(),
                });
            }
            path.push(component);
        }
        path.set_extension("class");

        std::fs::read(&path).map_err(|_| LoadError::NotFound {
            name: name.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_prevent_directory_traversal() {
        let root = std::env::temp_dir().join("duke_loader_tests");
        std::fs::create_dir_all(&root).unwrap();

        let secret_file = root.parent().unwrap().join("secret.class");
        std::fs::write(&secret_file, "SENSITIVE_DATA").unwrap();

        let loader = DirectoryLoader::new(&root);

        // Exploit: try to read outside root.
        let result = loader.find_class("../secret");

        std::fs::remove_file(&secret_file).unwrap();
        std::fs::remove_dir_all(&root).unwrap();

        // If it successfully reads "SENSITIVE_DATA", we have a vulnerability.
        // Red Phase: Ensure that it returns an error instead!
        assert!(
            matches!(result, Err(LoadError::NotFound { .. })),
            "Vulnerability triggered! Got {result:?}"
        );
    }

    #[test]
    fn should_prevent_absolute_paths_windows() {
        let loader = DirectoryLoader::new(std::path::PathBuf::from("/tmp"));
        let result = loader.find_class("C:\\Windows\\System32\\cmd");
        assert!(
            matches!(result, Err(LoadError::NotFound { .. })),
            "Vulnerability triggered! Got {result:?}"
        );
    }
}
