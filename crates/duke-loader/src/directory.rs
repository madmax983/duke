//! `duke-loader::directory` - Loads classes and resources from directories.

use std::path::{Path, PathBuf};

use crate::{ClassLoader, Error, LocatedResource, Result, path_to_file_url};

/// Loads `.class` files from a filesystem directory.
///
/// Given `find_class("java/lang/Object")`, looks for
/// `{root}/java/lang/Object.class`.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use duke_loader::{ClassLoader, DirectoryLoader};
///
/// let loader = DirectoryLoader::new(PathBuf::from("my_classes"));
/// // This will look for "my_classes/com/example/Main.class"
/// let _result = loader.find_class("com/example/Main");
/// ```
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

    fn resolve_child_path(&self, name: &str) -> Result<PathBuf> {
        if name.contains("..")
            || name.starts_with('/')
            || name.starts_with('\\')
            || name.contains(':')
        {
            return Err(Error::NotFound {
                name: name.to_string(),
            });
        }

        let mut path = self.root.clone();
        for component in name.split(['/', '\\']) {
            if component.is_empty() || component == "." || component == ".." {
                return Err(Error::NotFound {
                    name: name.to_string(),
                });
            }
            path.push(component);
        }
        Ok(path)
    }
}

impl ClassLoader for DirectoryLoader {
    fn find_class(&self, name: &str) -> Result<Vec<u8>> {
        if name.contains('.') {
            return Err(Error::NotFound {
                name: name.to_string(),
            });
        }
        let mut path = self.resolve_child_path(name)?;
        path.set_extension("class");

        std::fs::read(&path).map_err(|_| Error::NotFound {
            name: name.to_string(),
        })
    }

    fn find_resource(&self, name: &str) -> Result<Vec<u8>> {
        let path = self.resolve_child_path(name)?;
        std::fs::read(&path).map_err(|_| Error::NotFound {
            name: name.to_string(),
        })
    }

    fn find_resource_entry(&self, name: &str) -> Result<LocatedResource> {
        let path = self.resolve_child_path(name)?;
        let bytes = std::fs::read(&path).map_err(|_| Error::NotFound {
            name: name.to_string(),
        })?;
        Ok(LocatedResource {
            bytes,
            url: path_to_file_url(&path),
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

        let result = loader.find_class("../secret");

        std::fs::remove_file(&secret_file).unwrap();
        std::fs::remove_dir_all(&root).unwrap();

        assert!(
            matches!(result, Err(Error::NotFound { .. })),
            "Vulnerability triggered! Got {result:?}"
        );
    }

    #[test]
    fn should_prevent_absolute_paths_windows() {
        let loader = DirectoryLoader::new(std::path::PathBuf::from("/tmp"));
        let result = loader.find_class("C:\\Windows\\System32\\cmd");
        assert!(
            matches!(result, Err(Error::NotFound { .. })),
            "Vulnerability triggered! Got {result:?}"
        );
    }

    #[test]
    fn directory_loader_returns_not_found_on_empty_component() {
        let loader = DirectoryLoader::new(std::path::PathBuf::from("/tmp"));
        let err = loader.find_class("java//lang/Object").unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[test]
    fn directory_loader_reads_service_configuration_resource() {
        let root = std::env::temp_dir().join("duke_loader_service_resource");
        let services = root.join("META-INF").join("services");
        std::fs::create_dir_all(&services).unwrap();
        std::fs::write(services.join("com.example.Plugin"), "com.example.Impl\n").unwrap();

        let loader = DirectoryLoader::new(&root);
        let entries = loader
            .service_configuration_files("com.example.Plugin")
            .expect("read service file");

        std::fs::remove_dir_all(&root).unwrap();

        assert_eq!(entries, vec![b"com.example.Impl\n".to_vec()]);
    }

    #[test]
    fn directory_loader_rejects_resource_traversal() {
        let loader = DirectoryLoader::new(std::env::temp_dir());
        let result = loader.find_resource("../META-INF/services/evil");
        assert!(matches!(result, Err(Error::NotFound { .. })));
    }

    #[test]
    fn directory_loader_resource_entry_reports_file_url() {
        let root = std::env::temp_dir().join("duke_loader_resource_url");
        if root.exists() {
            std::fs::remove_dir_all(&root).unwrap();
        }
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("sample.txt"), b"hello").unwrap();

        let loader = DirectoryLoader::new(&root);
        let resource = loader
            .find_resource_entry("sample.txt")
            .expect("resource entry should resolve");

        std::fs::remove_dir_all(&root).unwrap();

        assert_eq!(resource.bytes, b"hello");
        assert!(
            resource.url.starts_with("file://"),
            "expected file URL, got {}",
            resource.url
        );
        assert!(resource.url.ends_with("sample.txt"));
    }
}
