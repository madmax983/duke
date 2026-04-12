use std::path::Path;

use super::reader::ZipReader;
use crate::{ClassLoader, LoadError, LoadResult};
// ───────────────────────────────────────────────────────────────────────────
// ZipLoader — ClassLoader over a JAR/ZIP
// ───────────────────────────────────────────────────────────────────────────

/// Loads `.class` files from a ZIP or JAR archive.
///
/// `ZipLoader` implements the [`ClassLoader`] trait to seamlessly find and read `.class`
/// files embedded inside a `.zip` or `.jar` archive. It uses a read-only, memory-mapped
/// [`ZipReader`] underneath to avoid eagerly unpacking the archive into memory.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use duke_loader::{ClassLoader, ZipLoader};
///
/// // 1. Open the archive
/// let loader = ZipLoader::open(Path::new("my_library.jar"))
///     .expect("Failed to open jar file");
///
/// // 2. Find a class by its internal JVM name
/// let bytes = loader.find_class("com/example/MyClass")
///     .expect("Class not found in archive");
///
/// assert_eq!(&bytes[0..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
/// ```
pub struct ZipLoader {
    reader: ZipReader,
    nested_libs: Vec<Self>,
}

impl ZipLoader {
    /// Open a ZIP/JAR file as a class loader.
    ///
    /// This immediately memory-maps the file and parses its Central Directory to build
    /// a fast lookup index. It does **not** decompress the file contents yet.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError`] if:
    /// * The file does not exist or cannot be read.
    /// * The file is not a structurally valid ZIP archive (missing End of Central Directory).
    /// * The archive uses unsupported features (like ZIP64 or encryption).
    pub fn open(path: &Path) -> LoadResult<Self> {
        Self::from_reader(ZipReader::open(path)?)
    }

    /// Access the underlying reader.
    ///
    /// This is particularly useful for reading non-class resources stored in the archive,
    /// such as the `META-INF/MANIFEST.MF` file or native libraries.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use duke_loader::{ZipLoader};
    ///
    /// let loader = ZipLoader::open(Path::new("app.jar")).unwrap();
    /// let manifest_bytes = loader.reader().read_entry("META-INF/MANIFEST.MF")
    ///     .expect("Missing manifest");
    /// ```
    #[must_use]
    pub const fn reader(&self) -> &ZipReader {
        &self.reader
    }

    pub(crate) fn from_reader(reader: ZipReader) -> LoadResult<Self> {
        let nested_libs = nested_boot_inf_lib_loaders(&reader)?;
        Ok(Self {
            reader,
            nested_libs,
        })
    }
}

impl ClassLoader for ZipLoader {
    fn find_class(&self, name: &str) -> LoadResult<Vec<u8>> {
        // Pre-allocate a single buffer large enough for the longest path
        // "BOOT-INF/classes/".len() == 17, ".class".len() == 6. Total = 23
        let mut entry_name = String::with_capacity(name.len() + 23);

        // Try standard class path: {name}.class
        entry_name.push_str(name);
        entry_name.push_str(".class");
        match self.reader.read_entry(&entry_name) {
            Err(LoadError::NotFound { .. }) => {}
            result => return result,
        }

        // Try BOOT-INF path: BOOT-INF/classes/{name}.class
        entry_name.clear();
        entry_name.push_str("BOOT-INF/classes/");
        entry_name.push_str(name);
        entry_name.push_str(".class");
        match self.reader.read_entry(&entry_name) {
            Err(LoadError::NotFound { .. }) => {}
            result => return result,
        }

        for nested_lib in &self.nested_libs {
            match nested_lib.find_class(name) {
                Err(LoadError::NotFound { .. }) => {}
                result => return result,
            }
        }
        Err(LoadError::NotFound {
            name: name.to_string(),
        })
    }
}

fn nested_boot_inf_lib_loaders(reader: &ZipReader) -> LoadResult<Vec<ZipLoader>> {
    let mut nested_entry_names: Vec<&str> = reader
        .entry_names()
        .filter(|name| is_nested_boot_inf_lib_archive(name))
        .collect();
    nested_entry_names.sort_unstable();

    let mut nested_libs = Vec::with_capacity(nested_entry_names.len());
    for entry_name in nested_entry_names {
        let nested_bytes = reader.read_entry(entry_name)?;
        nested_libs.push(ZipLoader::from_reader(ZipReader::from_bytes(
            nested_bytes,
        )?)?);
    }
    Ok(nested_libs)
}

fn is_nested_boot_inf_lib_archive(entry_name: &str) -> bool {
    let Some(suffix) = entry_name.strip_prefix("BOOT-INF/lib/") else {
        return false;
    };
    Path::new(suffix)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("jar") || ext.eq_ignore_ascii_case("zip"))
}
