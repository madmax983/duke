//! Reader for `OpenJDK` 21 jimage format (`lib/modules`).
//!
//! The jimage binary layout (little-endian throughout):
//!
//! ```text
//! Header (28 bytes):
//!   magic:           u32 = 0xCAFEDADA
//!   version:         u32 = 0x00010000 (major=1, minor=0)
//!   flags:           u32 = 0 (unused)
//!   resource_count:  u32
//!   table_length:    u32  (hash table size, prime >= resource_count)
//!   locations_size:  u32  (bytes in locations section)
//!   strings_size:    u32  (bytes in strings section)
//!
//! Index section (immediately after header):
//!   redirect:    [i32; table_length]   (perfect-hash redirect table)
//!   offsets:     [u32; table_length]   (location offsets)
//!   locations:   [u8; locations_size]  (location attribute streams)
//!   strings:     [u8; strings_size]    (null-terminated UTF-8 strings)
//!
//! Data section: starts at header_size + table_length*8 + locations_size + strings_size
//! ```
//!
//! **Location attribute encoding:**
//! Each location is a stream of attributes terminated by `END` (kind 0).
//! Each attribute: 1 header byte `(kind << 3) | (data_len - 1)` + data bytes (big-endian).
//!
//! Attribute kinds: END=0, MODULE=1, PARENT=2, BASE=3, EXTENSION=4,
//!                  OFFSET=5, COMPRESSED=6, UNCOMPRESSED=7.
//!
//! **String table:** null-terminated UTF-8 strings; index 0 = empty string.
//!
//! **Compression:** data may be raw deflate (no zlib header); COMPRESSED > 0 when active.

use std::{collections::HashMap, io::Read, path::Path};

use flate2::read::DeflateDecoder;

use crate::{ClassLoader, LoadError, LoadResult};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const JIMAGE_MAGIC: u32 = 0xCAFE_DADA;
const JIMAGE_VERSION: u32 = 0x0001_0000;
const HEADER_SIZE: usize = 28;

const ATTR_END: u8 = 0;
const ATTR_MODULE: u8 = 1;
const ATTR_PARENT: u8 = 2;
const ATTR_BASE: u8 = 3;
const ATTR_EXTENSION: u8 = 4;
const ATTR_OFFSET: u8 = 5;
const ATTR_COMPRESSED: u8 = 6;
const ATTR_UNCOMPRESSED: u8 = 7;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Metadata about a single resource stored in the jimage.
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    /// Byte offset into the data section.
    pub offset: u64,
    /// Compressed size (0 = not compressed, raw bytes).
    pub compressed: u64,
    /// Uncompressed (actual) size in bytes.
    pub uncompressed: u64,
}

/// Reader for `OpenJDK` 21 `lib/modules` jimage files.
///
/// On [`open`](JImageReader::open), reads the entire file into memory and
/// scans the locations table to build a path→resource index.  All subsequent
/// [`JImageReader::find_resource`] and [`JImageReader::read_resource`] calls are O(1) hash lookups.
pub struct JImageReader {
    data: Vec<u8>,
    resource_count: u32,
    data_offset: usize,
    /// Maps full jimage path `/module/parent/base.extension` → `ResourceInfo`.
    index: HashMap<String, ResourceInfo>,
}

// ---------------------------------------------------------------------------
// JImageReader impl
// ---------------------------------------------------------------------------

impl JImageReader {
    #[cfg(test)]
    pub(crate) fn empty_for_test() -> Self {
        Self {
            data: Vec::new(),
            resource_count: 0,
            data_offset: 0,
            index: HashMap::new(),
        }
    }

    /// Open and parse a jimage file, building the resource index.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError::Io`] if the file cannot be read, or
    /// [`LoadError::JImageFormat`] if the file is not a valid jimage.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use duke_loader::JImageReader;
    ///
    /// // Example of opening a potentially non-existent JDK `lib/modules` file.
    /// // In production, this path comes from the `JAVA_HOME` environment variable.
    /// let result = JImageReader::open(Path::new("/usr/lib/jvm/java-21-openjdk/lib/modules"));
    ///
    /// // It will gracefully return an Io error if the file is not found.
    /// assert!(result.is_err());
    /// ```
    pub fn open(path: &Path) -> LoadResult<Self> {
        let data = std::fs::read(path).map_err(|e| LoadError::Io {
            path: path.display().to_string(),
            source: e,
        })?;

        let (resource_count, table_length, locations_size, strings_size) = parse_header(&data)?;

        let tl = table_length as usize;
        let ls = locations_size as usize;
        let ss = strings_size as usize;

        // redirect: [i32; tl]  at HEADER_SIZE
        // offsets:  [u32; tl]  at HEADER_SIZE + tl*4
        // locations: [u8; ls]  at HEADER_SIZE + tl*8
        // strings:   [u8; ss]  at HEADER_SIZE + tl*8 + ls
        let locations_offset = HEADER_SIZE + tl * 8;
        let strings_offset = locations_offset + ls;
        let data_offset = strings_offset + ss;

        if data.len() < data_offset {
            return Err(LoadError::JImageFormat {
                msg: format!(
                    "file too small: {} bytes, need at least {}",
                    data.len(),
                    data_offset
                ),
            });
        }

        let index = build_index(
            &data,
            locations_offset,
            ls,
            strings_offset,
            resource_count as usize,
        );

        Ok(Self {
            data,
            resource_count,
            data_offset,
            index,
        })
    }

    /// Total number of resources declared in the jimage header.
    #[must_use]
    pub const fn resource_count(&self) -> u32 {
        self.resource_count
    }

    /// Look up a resource by its full jimage path.
    ///
    /// Path format: `"/module/parent/base.extension"`,
    /// e.g. `"/java.base/java/lang/Object.class"`.
    #[must_use]
    pub fn find_resource(&self, path: &str) -> Option<&ResourceInfo> {
        self.index.get(path)
    }

    /// Read and (if necessary) decompress a resource by its full jimage path.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError::NotFound`] if the path is not in the index,
    /// [`LoadError::JImageFormat`] if the data is out of bounds, or
    /// [`LoadError::Decompress`] if decompression fails.
    pub fn read_resource(&self, path: &str) -> LoadResult<Vec<u8>> {
        let info = self.index.get(path).ok_or_else(|| LoadError::NotFound {
            name: path.to_string(),
        })?;

        let raw_len = if info.compressed > 0 {
            usize::try_from(info.compressed).unwrap_or(usize::MAX)
        } else {
            usize::try_from(info.uncompressed).unwrap_or(usize::MAX)
        };
        let offset = usize::try_from(info.offset).unwrap_or(usize::MAX);
        let start =
            self.data_offset
                .checked_add(offset)
                .ok_or_else(|| LoadError::JImageFormat {
                    msg: format!("resource '{path}' offset overflow"),
                })?;

        if start
            .checked_add(raw_len)
            .is_none_or(|end| end > self.data.len())
        {
            return Err(LoadError::JImageFormat {
                msg: format!("resource '{path}' data out of bounds"),
            });
        }

        let raw = &self.data[start..start + raw_len];

        if info.compressed > 0 {
            // Raw deflate stream (negative window bits — no zlib header)
            let cap = usize::try_from(info.uncompressed).unwrap_or(0);
            let mut decoder = DeflateDecoder::new(raw);
            let mut out = Vec::with_capacity(cap.min(1024 * 1024 * 32));
            decoder
                .read_to_end(&mut out)
                .map_err(|_| LoadError::Decompress {
                    name: path.to_string(),
                })?;
            Ok(out)
        } else {
            Ok(raw.to_vec())
        }
    }
}

// ---------------------------------------------------------------------------
// ClassLoader impl — tries common modules in priority order
// ---------------------------------------------------------------------------

/// Modules to probe, in priority order. `java.base` covers the vast majority
/// of standard library classes; the rest cover common extensions.
const PROBE_MODULES: &[&str] = &[
    "java.base",
    "java.logging",
    "java.sql",
    "java.desktop",
    "java.management",
    "java.naming",
    "java.xml",
    "java.net.http",
    "jdk.compiler",
    "jdk.unsupported",
];

impl ClassLoader for JImageReader {
    fn find_class(&self, name: &str) -> LoadResult<Vec<u8>> {
        let (parent, base) = split_class_name(name);
        for module in PROBE_MODULES {
            let path = build_jimage_path(module, parent, base, "class");
            if self.index.contains_key(&path) {
                return self.read_resource(&path);
            }
        }
        Err(LoadError::NotFound {
            name: name.to_string(),
        })
    }
}

// ---------------------------------------------------------------------------
// Header parsing
// ---------------------------------------------------------------------------

/// Returns `(resource_count, table_length, locations_size, strings_size)`.
fn parse_header(data: &[u8]) -> LoadResult<(u32, u32, u32, u32)> {
    if data.len() < HEADER_SIZE {
        return Err(LoadError::JImageFormat {
            msg: format!("file too small for header: {} bytes", data.len()),
        });
    }

    let magic = read_u32_le(data, 0);
    if magic != JIMAGE_MAGIC {
        return Err(LoadError::JImageFormat {
            msg: format!("bad magic: {magic:#010x} (expected {JIMAGE_MAGIC:#010x})"),
        });
    }

    let version = read_u32_le(data, 4);
    if version != JIMAGE_VERSION {
        return Err(LoadError::JImageFormat {
            msg: format!("unsupported jimage version: {version:#010x}"),
        });
    }

    Ok((
        read_u32_le(data, 12), // resource_count
        read_u32_le(data, 16), // table_length
        read_u32_le(data, 20), // locations_size
        read_u32_le(data, 24), // strings_size
    ))
