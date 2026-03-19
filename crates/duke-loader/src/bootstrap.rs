use std::path::Path;

use crate::{ClassLoader, DirectoryLoader, JImageReader, LoadError, LoadResult};

/// Bootstrap class loader.
///
/// Resolves classes by trying the JDK jimage first (for standard library
/// classes), then falling through to classpath directories (for application
/// classes).
#[derive(Debug)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_mock_jimage(path: &Path) {
        // Build a minimal jimage that contains a resource for java/lang/Object.class
        let mut buf = vec![0u8; 28]; // HEADER_SIZE
        buf[0..4].copy_from_slice(&0xCAFE_DADA_u32.to_le_bytes()); // JIMAGE_MAGIC
        buf[4..8].copy_from_slice(&0x0001_0000_u32.to_le_bytes()); // JIMAGE_VERSION
        buf[12..16].copy_from_slice(&1u32.to_le_bytes()); // resource_count
        buf[16..20].copy_from_slice(&1u32.to_le_bytes()); // table_length
        buf[20..24].copy_from_slice(&12u32.to_le_bytes()); // locations_size (10 bytes locs + 2 padding)
        buf[24..28].copy_from_slice(&32u32.to_le_bytes()); // strings_size

        // String table: "\0java.base\0java/lang\0Object\0class\0"
        let strings: &[u8] = b"\x00java.base\x00java/lang\x00Object\x00class\x00"; // 34 bytes

        // module=1, parent=11, base=21, extension=28
        // offset=0, uncompressed=4 (4 bytes data)
        let locs: &[u8] = &[
            0x08, 0x01, // MODULE, len=1, val=1
            0x10, 0x0b, // PARENT, len=1, val=11
            0x18, 0x15, // BASE, len=1, val=21
            0x20, 0x1c, // EXTENSION, len=1, val=28
            0x38, 0x04, // UNCOMPRESSED, len=1, val=4
            0x28, 0x00, // OFFSET, len=1, val=0
        ]; // 12 bytes

        // redirect = [0]
        // offsets = [0]
        // locs = [12 bytes]
        // strings = [34 bytes]
        // data = b"java" (4 bytes)
        let redirect: &[u8] = &[0, 0, 0, 0];
        let offsets: &[u8] = &[0, 0, 0, 0];
        let data: &[u8] = b"java";

        let mut file_data = buf;
        file_data[24..28].copy_from_slice(&34u32.to_le_bytes()); // update strings_size to 34 bytes

        file_data.extend_from_slice(redirect);
        file_data.extend_from_slice(offsets);
        file_data.extend_from_slice(locs);
        // Pad locs up to 12 bytes as expected by the header if it wasn't exactly 12 bytes long.
        // the length of locs is exactly 12 so we don't need padding.
        file_data.extend_from_slice(strings);
        file_data.extend_from_slice(data);

        fs::write(path, &file_data).expect("failed to write mock jimage");
    }

    #[test]
    fn should_load_class_from_jimage_when_present() {
        let dir = tempdir().unwrap();
        let jimage_path = dir.path().join("modules");
        write_mock_jimage(&jimage_path);

        let loader = BootstrapLoader::new(&jimage_path, Vec::<&str>::new()).unwrap();
        let bytes = loader.find_class("java/lang/Object").unwrap();
        assert_eq!(bytes, b"java");
    }

    #[test]
    fn should_load_class_from_classpath_when_not_in_jimage() {
        let dir = tempdir().unwrap();
        let jimage_path = dir.path().join("modules");
        write_mock_jimage(&jimage_path);

        let cp_dir = dir.path().join("cp");
        fs::create_dir_all(cp_dir.join("com/example")).unwrap();
        fs::write(cp_dir.join("com/example/MyClass.class"), b"app_class").unwrap();

        let loader = BootstrapLoader::new(&jimage_path, vec![&cp_dir]).unwrap();
        let bytes = loader.find_class("com/example/MyClass").unwrap();
        assert_eq!(bytes, b"app_class");
    }

    #[test]
    fn should_return_error_when_class_not_found_anywhere() {
        let dir = tempdir().unwrap();
        let jimage_path = dir.path().join("modules");
        write_mock_jimage(&jimage_path);

        let cp_dir = dir.path().join("cp");
        fs::create_dir_all(&cp_dir).unwrap();

        let loader = BootstrapLoader::new(&jimage_path, vec![&cp_dir]).unwrap();
        let err = loader.find_class("com/example/MissingClass").unwrap_err();
        match err {
            LoadError::NotFound { name } => assert_eq!(name, "com/example/MissingClass"),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn should_return_error_when_jimage_cannot_be_opened() {
        let dir = tempdir().unwrap();
        let jimage_path = dir.path().join("missing_modules");

        let err = BootstrapLoader::new(&jimage_path, Vec::<&str>::new()).unwrap_err();
        match err {
            LoadError::Io { path, .. } => assert!(path.ends_with("missing_modules")),
            _ => panic!("Expected Io error"),
        }
    }
}
