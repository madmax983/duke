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
        for component in name.split('/') {
            path.push(component);
        }
        path.set_extension("class");

        std::fs::read(&path).map_err(|_| LoadError::NotFound {
            name: name.to_string(),
        })
    }
}
