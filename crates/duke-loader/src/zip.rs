//! Read-only ZIP/JAR archive support.
//!
//! Implements index-on-open, lazy decompression — the same pattern as
//! [`super::jimage::JImageReader`].  Used both by the internal classloader
//! (`ZipLoader`) and by the Java-space `ZipFile` native bridges.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use crate::{ClassLoader, Error, LocatedResource, Result, path_to_file_url};

// ───────────────────────────────────────────────────────────────────────────
// ZIP format constants
// ───────────────────────────────────────────────────────────────────────────

const EOCD_SIGNATURE: u32 = 0x0605_4b50; // PK\x05\x06
const CD_SIGNATURE: u32 = 0x0201_4b50; // PK\x01\x02
const LOCAL_SIGNATURE: u32 = 0x0403_4b50; // PK\x03\x04
const METHOD_STORED: u16 = 0;
const METHOD_DEFLATED: u16 = 8;

/// Minimum size of an EOCD record (no comment).
const EOCD_MIN_SIZE: usize = 22;

/// Maximum distance from end-of-file to scan for the EOCD signature.
/// (22-byte EOCD + up to 65535-byte comment.)
const EOCD_MAX_SEARCH: usize = EOCD_MIN_SIZE + 65535;

// ───────────────────────────────────────────────────────────────────────────
// CRC-32 (standard IEEE polynomial 0xEDB88320)
// ───────────────────────────────────────────────────────────────────────────

/// Build the CRC-32 lookup table at compile time.
#[allow(clippy::cast_possible_truncation)]
const CRC32_TABLE: [u32; 256] = {
    let mut table = [0_u32; 256];
    let mut i: usize = 0;
    while i < 256 {
        let mut crc = i as u32; // i ∈ 0..256, always fits u32
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
};

fn crc32_checksum(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        let idx = ((crc ^ u32::from(byte)) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32_TABLE[idx];
    }
    !crc
}

// ───────────────────────────────────────────────────────────────────────────
// Public types
// ───────────────────────────────────────────────────────────────────────────

/// Metadata for a single entry in the ZIP central directory.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use duke_loader::ZipReader;
///
/// let reader = ZipReader::open(Path::new("app.jar")).unwrap();
/// if let Some(info) = reader.get_entry("com/example/Main.class") {
///     println!("Found {}, compressed size: {}", info.name, info.compressed_size);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ZipEntryInfo {
    /// Entry name (e.g. `"com/example/Main.class"`).
    pub name: String,
    /// Compression method: 0 = STORED, 8 = DEFLATED.
    pub compression_method: u16,
    /// CRC-32 of the uncompressed data.
    pub crc32: u32,
    /// Compressed size in bytes.
    pub compressed_size: u64,
    /// Uncompressed size in bytes.
    pub uncompressed_size: u64,
    /// Offset of the corresponding local file header.
    local_header_offset: u64,
}

/// Read-only ZIP archive reader with index-on-open, lazy decompression.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use duke_loader::ZipReader;
///
/// let reader = ZipReader::open(Path::new("app.jar")).expect("failed to open jar");
/// println!("Found {} entries", reader.entry_count());
/// ```
#[derive(Debug)]
pub struct ZipReader {
    data: Vec<u8>,
    index: HashMap<String, ZipEntryInfo>,
}

impl ZipReader {
    /// Open a ZIP/JAR file from disk.
    ///
    /// Reads the entire file into memory, parses the end-of-central-directory
    /// record and central directory, and builds an in-memory index.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use duke_loader::ZipReader;
    ///
    /// let reader = ZipReader::open(Path::new("app.jar")).unwrap();
    /// ```
    ///
    /// # Errors
    /// Returns [`Error::Io`] on read failure, or [`Error::ZipFormat`]
    /// if the file is not a valid ZIP archive.
    pub fn open(path: &Path) -> Result<Self> {
        let data = std::fs::read(path).map_err(|source| Error::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_bytes(data)
    }

    /// Build a `ZipReader` from raw bytes (useful for tests).
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_loader::ZipReader;
    ///
    /// let archive_data = vec![
    ///     0x50, 0x4b, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00,
    ///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    /// ];
    /// let reader = ZipReader::from_bytes(archive_data).unwrap();
    /// ```
    ///
    /// # Errors
    /// Returns [`Error::ZipFormat`] if the data is not a valid ZIP archive.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        let eocd_pos = find_eocd(&data)?;
        let index = parse_eocd_and_central_directory(&data, eocd_pos)?;
        Ok(Self { data, index })
    }

    /// Look up an entry by name.  O(1).
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_loader::ZipReader;
    ///
    /// let archive_data = vec![
    ///     0x50, 0x4b, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00,
    ///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    /// ];
    /// let reader = ZipReader::from_bytes(archive_data).unwrap();
    /// assert!(reader.get_entry("nonexistent.txt").is_none());
    /// ```
    #[must_use]
    pub fn get_entry(&self, name: &str) -> Option<&ZipEntryInfo> {
        self.index.get(name)
    }

    /// Decompress and return the bytes for the named entry.
    ///
    /// Validates the CRC-32 after decompression.
    ///
    /// # Errors
    /// Returns [`Error::NotFound`] if the entry doesn't exist,
    /// [`Error::ZipFormat`] on decompression failure, or
    /// [`Error::ZipCrc32`] on checksum mismatch.
    pub fn read_entry(&self, name: &str) -> Result<Vec<u8>> {
        let info = self.index.get(name).ok_or_else(|| Error::NotFound {
            name: name.to_string(),
        })?;
        self.read_entry_info(info)
    }

    /// Read entry bytes given a pre-looked-up `ZipEntryInfo`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use duke_loader::ZipReader;
    ///
    /// let reader = ZipReader::open(Path::new("app.jar")).unwrap();
    /// if let Some(info) = reader.get_entry("file.txt") {
    ///     let data = reader.read_entry_info(info).unwrap();
    /// }
    /// ```
    ///
    /// # Errors
    /// Returns [`Error::ZipFormat`] on decompression or format errors,
    /// or [`Error::ZipCrc32`] on checksum mismatch.
    #[allow(clippy::cast_possible_truncation)]
    pub fn read_entry_info(&self, info: &ZipEntryInfo) -> Result<Vec<u8>> {
        let offset = usize::try_from(info.local_header_offset).unwrap_or(usize::MAX);

        // Validate local file header signature.
        if offset
            .checked_add(30)
            .is_none_or(|end| end > self.data.len())
        {
            return Err(Error::ZipFormat {
                msg: format!("local header at offset {offset} is truncated"),
            });
        }
        let sig = read_u32_le(&self.data, offset);
        if sig != LOCAL_SIGNATURE {
            return Err(Error::ZipFormat {
                msg: format!("expected local header signature at offset {offset}, got {sig:#010x}"),
            });
        }

        // Read local header's own filename_len and extra_len to find data start.
        if offset
            .checked_add(30)
            .is_none_or(|end| end > self.data.len())
        {
            return Err(Error::ZipFormat {
                msg: format!("local header at offset {offset} is truncated"),
            });
        }
        let filename_len = usize::from(read_u16_le(&self.data, offset + 26));
        let extra_len = usize::from(read_u16_le(&self.data, offset + 28));
        let data_start = offset
            .checked_add(30)
            .and_then(|v| v.checked_add(filename_len))
            .and_then(|v| v.checked_add(extra_len))
            .ok_or_else(|| Error::ZipFormat {
                msg: format!("entry '{}' local header offset overflow", info.name),
            })?;

        let compressed_size = usize::try_from(info.compressed_size).unwrap_or(usize::MAX);

        if data_start
            .checked_add(compressed_size)
            .is_none_or(|end| end > self.data.len())
        {
            return Err(Error::ZipFormat {
                msg: format!("entry '{}' data extends past end of archive", info.name),
            });
        }

        let compressed = &self.data[data_start..data_start + compressed_size];

        let decompressed = match info.compression_method {
            METHOD_STORED => compressed.to_vec(),
            METHOD_DEFLATED => {
                let decoder = flate2::read::DeflateDecoder::new(compressed);
                let cap = usize::try_from(info.uncompressed_size).unwrap_or(usize::MAX);
                let max_size = 1024 * 1024 * 256; // 256 MB max size to prevent OOM
                if cap > max_size {
                    return Err(Error::ZipFormat {
                        msg: format!(
                            "entry '{}' uncompressed size {} exceeds limit {}",
                            info.name, cap, max_size
                        ),
                    });
                }
                let mut buf = Vec::with_capacity(cap.min(compressed.len().saturating_mul(2)));
                let bytes_read = decoder
                    .take((max_size as u64).saturating_add(1))
                    .read_to_end(&mut buf)
                    .map_err(|_| Error::ZipFormat {
                        msg: format!("failed to deflate entry '{}'", info.name),
                    })?;

                if bytes_read > max_size {
                    return Err(Error::ZipFormat {
                        msg: format!(
                            "entry '{}' uncompressed size exceeds limit {}",
                            info.name, max_size
                        ),
                    });
                }

                buf
            }
            other => {
                return Err(Error::ZipFormat {
                    msg: format!(
                        "unsupported compression method {other} for entry '{}'",
                        info.name
                    ),
                });
            }
        };

        // Validate CRC-32.
        let actual_crc = crc32_checksum(&decompressed);
        if actual_crc != info.crc32 {
            return Err(Error::ZipCrc32 {
                name: info.name.clone(),
                expected: info.crc32,
                actual: actual_crc,
            });
        }

        Ok(decompressed)
    }

    /// Number of entries in the archive.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_loader::ZipReader;
    ///
    /// let archive_data = vec![
    ///     0x50, 0x4b, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00,
    ///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    /// ];
    /// let reader = ZipReader::from_bytes(archive_data).unwrap();
    /// assert_eq!(reader.entry_count(), 0);
    /// ```
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.index.len()
    }

    /// Iterate over all entry names.
    ///
    /// # Examples
    ///
    /// ```
    /// use duke_loader::ZipReader;
    ///
    /// let archive_data = vec![
    ///     0x50, 0x4b, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00,
    ///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    /// ];
    /// let reader = ZipReader::from_bytes(archive_data).unwrap();
    /// assert_eq!(reader.entry_names().count(), 0);
    /// ```
    pub fn entry_names(&self) -> impl Iterator<Item = &str> {
        self.index.keys().map(String::as_str)
    }
}

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
    container_spec: String,
    nested_libs: Vec<Self>,
}

impl ZipLoader {
    #[inline]
    fn append_boot_inf_classes_path(buf: &mut String, name: &str) {
        buf.push_str("BOOT-INF/classes/");
        buf.push_str(name);
    }

    /// Open a ZIP/JAR file as a class loader.
    ///
    /// This immediately memory-maps the file and parses its Central Directory to build
    /// a fast lookup index. It does **not** decompress the file contents yet.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if:
    /// * The file does not exist or cannot be read.
    /// * The file is not a structurally valid ZIP archive (missing End of Central Directory).
    /// * The archive uses unsupported features (like ZIP64 or encryption).
    pub fn open(path: &Path) -> Result<Self> {
        let container_spec = path_to_file_url(path);
        Self::from_reader(ZipReader::open(path)?, container_spec)
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

    fn from_reader(reader: ZipReader, container_spec: String) -> Result<Self> {
        let nested_libs = nested_boot_inf_lib_loaders(&reader, &container_spec)?;
        Ok(Self {
            reader,
            container_spec,
            nested_libs,
        })
    }
    /// ⚡ Bolt: Eliminates intermediate String allocation and `format!` macro overhead
    /// by pre-computing string capacity and using `.push_str()` sequentially.
    fn resource_url(&self, entry_name: &str) -> String {
        let mut url = String::with_capacity(4 + self.container_spec.len() + 2 + entry_name.len());
        url.push_str("jar:");
        url.push_str(&self.container_spec);
        url.push_str("!/");
        url.push_str(entry_name);
        url
    }
}

/// Helper function to flatten zip entry lookups. Returns None if the entry is not found,
/// allowing easy chaining with `if let Some(res) = ignore_not_found(...)`
fn ignore_not_found<T>(res: Result<T>) -> Option<Result<T>> {
    match res {
        Err(Error::NotFound { .. }) => None,
        other => Some(other),
    }
}

impl ClassLoader for ZipLoader {
    fn find_class(&self, name: &str) -> Result<Vec<u8>> {
        // Pre-allocate a single buffer large enough for the longest path
        // "BOOT-INF/classes/".len() == 17, ".class".len() == 6. Total = 23
        let mut entry_name = String::with_capacity(name.len() + 23);

        // Try standard class path: {name}.class
        entry_name.push_str(name);
        entry_name.push_str(".class");
        if let Some(result) = ignore_not_found(self.reader.read_entry(&entry_name)) {
            return result;
        }

        // Try BOOT-INF path: BOOT-INF/classes/{name}.class
        entry_name.clear();
        Self::append_boot_inf_classes_path(&mut entry_name, name);
        entry_name.push_str(".class");
        if let Some(result) = ignore_not_found(self.reader.read_entry(&entry_name)) {
            return result;
        }

        for nested_lib in &self.nested_libs {
            if let Some(result) = ignore_not_found(nested_lib.find_class(name)) {
                return result;
            }
        }
        Err(Error::NotFound {
            name: name.to_string(),
        })
    }

    fn find_resource(&self, name: &str) -> Result<Vec<u8>> {
        if let Some(result) = ignore_not_found(self.reader.read_entry(name)) {
            return result;
        }

        // ⚡ Bolt: Eliminate intermediate String allocation and format! macro overhead
        let mut boot_inf_name = String::with_capacity(name.len() + 17);
        Self::append_boot_inf_classes_path(&mut boot_inf_name, name);
        if let Some(result) = ignore_not_found(self.reader.read_entry(&boot_inf_name)) {
            return result;
        }

        for nested_lib in &self.nested_libs {
            if let Some(result) = ignore_not_found(nested_lib.find_resource(name)) {
                return result;
            }
        }
        Err(Error::NotFound {
            name: name.to_string(),
        })
    }

    fn find_resources(&self, name: &str) -> Result<Vec<Vec<u8>>> {
        let mut resources = Vec::new();
        if let Some(result) = ignore_not_found(self.reader.read_entry(name)) {
            resources.push(result?);
        }

        // ⚡ Bolt: Eliminate intermediate String allocation and format! macro overhead
        let mut boot_inf_name = String::with_capacity(name.len() + 17);
        Self::append_boot_inf_classes_path(&mut boot_inf_name, name);
        if let Some(result) = ignore_not_found(self.reader.read_entry(&boot_inf_name)) {
            resources.push(result?);
        }

        for nested_lib in &self.nested_libs {
            resources.extend(nested_lib.find_resources(name)?);
        }
        Ok(resources)
    }

    fn find_resource_entry(&self, name: &str) -> Result<LocatedResource> {
        if let Some(result) = ignore_not_found(self.reader.read_entry(name)) {
            return Ok(LocatedResource {
                bytes: result?,
                url: self.resource_url(name),
            });
        }

        let mut boot_inf_name = String::with_capacity(name.len() + 17);
        Self::append_boot_inf_classes_path(&mut boot_inf_name, name);
        if let Some(result) = ignore_not_found(self.reader.read_entry(&boot_inf_name)) {
            return Ok(LocatedResource {
                bytes: result?,
                url: self.resource_url(&boot_inf_name),
            });
        }

        for nested_lib in &self.nested_libs {
            if let Some(result) = ignore_not_found(nested_lib.find_resource_entry(name)) {
                return result;
            }
        }

        Err(Error::NotFound {
            name: name.to_string(),
        })
    }

    fn find_resource_entries(&self, name: &str) -> Result<Vec<LocatedResource>> {
        let mut resources = Vec::new();
        if let Some(result) = ignore_not_found(self.reader.read_entry(name)) {
            resources.push(LocatedResource {
                bytes: result?,
                url: self.resource_url(name),
            });
        }

        let mut boot_inf_name = String::with_capacity(name.len() + 17);
        Self::append_boot_inf_classes_path(&mut boot_inf_name, name);
        if let Some(result) = ignore_not_found(self.reader.read_entry(&boot_inf_name)) {
            resources.push(LocatedResource {
                bytes: result?,
                url: self.resource_url(&boot_inf_name),
            });
        }

        for nested_lib in &self.nested_libs {
            resources.extend(nested_lib.find_resource_entries(name)?);
        }

        Ok(resources)
    }
}

fn nested_boot_inf_lib_loaders(reader: &ZipReader, container_spec: &str) -> Result<Vec<ZipLoader>> {
    let mut nested_entry_names: Vec<&str> = reader
        .entry_names()
        .filter(|name| is_nested_boot_inf_lib_archive(name))
        .collect();
    nested_entry_names.sort_unstable();

    let mut nested_libs = Vec::with_capacity(nested_entry_names.len());
    for entry_name in nested_entry_names {
        let nested_bytes = reader.read_entry(entry_name)?;

        let mut nested_container_spec =
            String::with_capacity(container_spec.len() + 2 + entry_name.len());
        nested_container_spec.push_str(container_spec);
        nested_container_spec.push_str("!/");
        nested_container_spec.push_str(entry_name);

        nested_libs.push(ZipLoader::from_reader(
            ZipReader::from_bytes(nested_bytes)?,
            nested_container_spec,
        )?);
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

// ───────────────────────────────────────────────────────────────────────────
// Private parsing helpers
// ───────────────────────────────────────────────────────────────────────────

/// Little-endian u16 read.
fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

/// Little-endian u32 read.
fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

/// Scan backwards from end of file to find the EOCD signature.
fn find_eocd(data: &[u8]) -> Result<usize> {
    if data.len() < EOCD_MIN_SIZE {
        return Err(Error::ZipFormat {
            msg: "file too small to be a valid ZIP archive".to_string(),
        });
    }
    let search_start = data.len().saturating_sub(EOCD_MAX_SEARCH);
    // Scan backwards; the EOCD is typically at the very end.
    let mut pos = data.len() - EOCD_MIN_SIZE;
    loop {
        if read_u32_le(data, pos) == EOCD_SIGNATURE {
            return Ok(pos);
        }
        if pos == search_start {
            break;
        }
        pos -= 1;
    }
    Err(Error::ZipFormat {
        msg: "could not find end-of-central-directory record".to_string(),
    })
}

/// Parse the EOCD record and then the central directory it points to.
fn parse_eocd_and_central_directory(
    data: &[u8],
    eocd_pos: usize,
) -> Result<HashMap<String, ZipEntryInfo>> {
    // EOCD layout (22 bytes minimum):
    //  0: signature (4)
    //  4: disk number (2)
    //  6: disk where CD starts (2)
    //  8: number of CD entries on this disk (2)
    // 10: total number of CD entries (2)
    // 12: size of CD (4)
    // 16: offset of start of CD (4)
    // 20: comment length (2)
    let entry_count = usize::from(read_u16_le(data, eocd_pos + 10));
    let cd_size = usize::try_from(read_u32_le(data, eocd_pos + 12)).unwrap_or(usize::MAX);
    let cd_offset = usize::try_from(read_u32_le(data, eocd_pos + 16)).unwrap_or(usize::MAX);

    if cd_offset
        .checked_add(cd_size)
        .is_none_or(|end| end > data.len())
    {
        return Err(Error::ZipFormat {
            msg: "central directory extends past end of file".to_string(),
        });
    }

    parse_central_directory(data, cd_offset, cd_size, entry_count)
}

/// Parse central directory entries into a `HashMap`.
struct CdHeader {
    compression_method: u16,
    crc32: u32,
    compressed_size: u64,
    uncompressed_size: u64,
    filename_len: usize,
    extra_len: usize,
    comment_len: usize,
    local_header_offset: u64,
}

fn parse_cd_header(data: &[u8], pos: usize) -> CdHeader {
    CdHeader {
        compression_method: read_u16_le(data, pos + 10),
        crc32: read_u32_le(data, pos + 16),
        compressed_size: u64::from(read_u32_le(data, pos + 20)),
        uncompressed_size: u64::from(read_u32_le(data, pos + 24)),
        filename_len: usize::from(read_u16_le(data, pos + 28)),
        extra_len: usize::from(read_u16_le(data, pos + 30)),
        comment_len: usize::from(read_u16_le(data, pos + 32)),
        local_header_offset: u64::from(read_u32_le(data, pos + 42)),
    }
}

fn parse_central_directory(
    data: &[u8],
    cd_offset: usize,
    cd_size: usize,
    expected_count: usize,
) -> Result<HashMap<String, ZipEntryInfo>> {
    let safe_capacity = expected_count.min(cd_size / 46);
    let mut index = HashMap::with_capacity(safe_capacity);
    let cd_end = cd_offset + cd_size;
    let mut pos = cd_offset;

    for _ in 0..expected_count {
        // Each central directory header is at least 46 bytes.
        if pos.checked_add(46).is_none_or(|end| end > cd_end) {
            return Err(Error::ZipFormat {
                msg: "central directory entry truncated".to_string(),
            });
        }
        let sig = read_u32_le(data, pos);
        if sig != CD_SIGNATURE {
            return Err(Error::ZipFormat {
                msg: format!(
                    "expected central directory signature at offset {pos}, got {sig:#010x}"
                ),
            });
        }

        let hdr = parse_cd_header(data, pos);

        let name_start = pos + 46;
        if name_start
            .checked_add(hdr.filename_len)
            .is_none_or(|end| end > cd_end)
        {
            return Err(Error::ZipFormat {
                msg: "central directory entry filename truncated".to_string(),
            });
        }

        let name =
            String::from_utf8_lossy(&data[name_start..name_start + hdr.filename_len]).into_owned();

        index.insert(
            name.clone(),
            ZipEntryInfo {
                name,
                compression_method: hdr.compression_method,
                crc32: hdr.crc32,
                compressed_size: hdr.compressed_size,
                uncompressed_size: hdr.uncompressed_size,
                local_header_offset: hdr.local_header_offset,
            },
        );

        pos = name_start
            .checked_add(hdr.filename_len)
            .and_then(|v| v.checked_add(hdr.extra_len))
            .and_then(|v| v.checked_add(hdr.comment_len))
            .ok_or_else(|| Error::ZipFormat {
                msg: "central directory entry length overflow".to_string(),
            })?;

        if pos > cd_end {
            return Err(Error::ZipFormat {
                msg: "central directory entry extends past CD bounds".to_string(),
            });
        }
    }

    Ok(index)
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::cast_possible_truncation)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_loader_reader() {
        let zip_bytes = build_stored_zip("test.txt", b"hello world");
        let reader = ZipReader::from_bytes(zip_bytes).expect("valid zip");
        let loader = ZipLoader::from_reader(reader, "file://memory/test.zip".to_string())
            .expect("valid zip");
        let r = loader.reader();
        assert_eq!(r.data.len(), 125);
    }

    #[test]
    fn zip_loader_resource_entry_reports_jar_url() {
        let zip_bytes = build_stored_zip("META-INF/messages.txt", b"hello");
        let reader = ZipReader::from_bytes(zip_bytes).expect("valid zip");
        let loader = ZipLoader::from_reader(reader, "file://memory/test.zip".to_string())
            .expect("valid zip");
        let resource = loader
            .find_resource_entry("META-INF/messages.txt")
            .expect("resource entry should resolve");

        assert_eq!(resource.bytes, b"hello");
        assert_eq!(
            resource.url,
            "jar:file://memory/test.zip!/META-INF/messages.txt"
        );
    }

    // ── Helpers: build minimal valid ZIPs in memory ──────────────────────

    /// Build a minimal ZIP archive containing one STORED entry.
    fn build_stored_zip(name: &str, content: &[u8]) -> Vec<u8> {
        let crc = crc32_checksum(content);
        let size = content.len() as u32;
        let name_bytes = name.as_bytes();

        let cap = 30 + name_bytes.len() + content.len() + 46 + name_bytes.len() + 22;
        let mut zip = Vec::with_capacity(cap);

        // ── Local file header ────────────────────────────────────────
        let local_offset = zip.len() as u32;
        zip.extend_from_slice(&LOCAL_SIGNATURE.to_le_bytes());
        zip.extend_from_slice(&20_u16.to_le_bytes()); // version needed
        zip.extend_from_slice(&0_u16.to_le_bytes()); // general purpose flags
        zip.extend_from_slice(&METHOD_STORED.to_le_bytes()); // compression
        zip.extend_from_slice(&0_u16.to_le_bytes()); // mod time
        zip.extend_from_slice(&0_u16.to_le_bytes()); // mod date
        zip.extend_from_slice(&crc.to_le_bytes());
        zip.extend_from_slice(&size.to_le_bytes()); // compressed size
        zip.extend_from_slice(&size.to_le_bytes()); // uncompressed size
        zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes()); // extra field len
        zip.extend_from_slice(name_bytes);
        zip.extend_from_slice(content);

        // ── Central directory ────────────────────────────────────────
        let cd_offset = zip.len() as u32;
        zip.extend_from_slice(&CD_SIGNATURE.to_le_bytes());
        zip.extend_from_slice(&20_u16.to_le_bytes()); // version made by
        zip.extend_from_slice(&20_u16.to_le_bytes()); // version needed
        zip.extend_from_slice(&0_u16.to_le_bytes()); // flags
        zip.extend_from_slice(&METHOD_STORED.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes()); // mod time
        zip.extend_from_slice(&0_u16.to_le_bytes()); // mod date
        zip.extend_from_slice(&crc.to_le_bytes());
        zip.extend_from_slice(&size.to_le_bytes());
        zip.extend_from_slice(&size.to_le_bytes());
        zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes()); // extra field len
        zip.extend_from_slice(&0_u16.to_le_bytes()); // comment len
        zip.extend_from_slice(&0_u16.to_le_bytes()); // disk number start
        zip.extend_from_slice(&0_u16.to_le_bytes()); // internal attrs
        zip.extend_from_slice(&0_u32.to_le_bytes()); // external attrs
        zip.extend_from_slice(&local_offset.to_le_bytes());
        zip.extend_from_slice(name_bytes);
        let cd_size = (zip.len() as u32) - cd_offset;

        // ── EOCD ─────────────────────────────────────────────────────
        zip.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes()); // disk number
        zip.extend_from_slice(&0_u16.to_le_bytes()); // disk with CD
        zip.extend_from_slice(&1_u16.to_le_bytes()); // entries on disk
        zip.extend_from_slice(&1_u16.to_le_bytes()); // total entries
        zip.extend_from_slice(&cd_size.to_le_bytes());
        zip.extend_from_slice(&cd_offset.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes()); // comment len

        zip
    }

    /// Build a ZIP with one DEFLATED entry.
    fn build_deflated_zip(name: &str, content: &[u8]) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::DeflateEncoder;
        use std::io::Write;

        let crc = crc32_checksum(content);
        let uncompressed_size = content.len() as u32;

        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(content).unwrap();
        let compressed = encoder.finish().unwrap();
        let compressed_size = compressed.len() as u32;
        let name_bytes = name.as_bytes();

        let cap = 30 + name_bytes.len() + compressed.len() + 46 + name_bytes.len() + 22;
        let mut zip = Vec::with_capacity(cap);

        // ── Local file header ────────────────────────────────────────
        let local_offset = zip.len() as u32;
        zip.extend_from_slice(&LOCAL_SIGNATURE.to_le_bytes());
        zip.extend_from_slice(&20_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&METHOD_DEFLATED.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&crc.to_le_bytes());
        zip.extend_from_slice(&compressed_size.to_le_bytes());
        zip.extend_from_slice(&uncompressed_size.to_le_bytes());
        zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(name_bytes);
        zip.extend_from_slice(&compressed);

        // ── Central directory ────────────────────────────────────────
        let cd_offset = zip.len() as u32;
        zip.extend_from_slice(&CD_SIGNATURE.to_le_bytes());
        zip.extend_from_slice(&20_u16.to_le_bytes());
        zip.extend_from_slice(&20_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&METHOD_DEFLATED.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&crc.to_le_bytes());
        zip.extend_from_slice(&compressed_size.to_le_bytes());
        zip.extend_from_slice(&uncompressed_size.to_le_bytes());
        zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u32.to_le_bytes());
        zip.extend_from_slice(&local_offset.to_le_bytes());
        zip.extend_from_slice(name_bytes);
        let cd_size = (zip.len() as u32) - cd_offset;

        // ── EOCD ─────────────────────────────────────────────────────
        zip.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&1_u16.to_le_bytes());
        zip.extend_from_slice(&1_u16.to_le_bytes());
        zip.extend_from_slice(&cd_size.to_le_bytes());
        zip.extend_from_slice(&cd_offset.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());

        zip
    }

    /// Build a ZIP with multiple STORED entries.
    fn build_multi_entry_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let count = entries.len() as u16;
        let mut zip = Vec::new();
        let mut local_offsets = Vec::with_capacity(entries.len());

        // ── Local file headers + data ────────────────────────────────
        for &(name, content) in entries {
            let crc = crc32_checksum(content);
            let size = content.len() as u32;
            let name_bytes = name.as_bytes();

            local_offsets.push(zip.len() as u32);
            zip.extend_from_slice(&LOCAL_SIGNATURE.to_le_bytes());
            zip.extend_from_slice(&20_u16.to_le_bytes());
            zip.extend_from_slice(&0_u16.to_le_bytes());
            zip.extend_from_slice(&METHOD_STORED.to_le_bytes());
            zip.extend_from_slice(&0_u16.to_le_bytes());
            zip.extend_from_slice(&0_u16.to_le_bytes());
            zip.extend_from_slice(&crc.to_le_bytes());
            zip.extend_from_slice(&size.to_le_bytes());
            zip.extend_from_slice(&size.to_le_bytes());
            zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            zip.extend_from_slice(&0_u16.to_le_bytes());
            zip.extend_from_slice(name_bytes);
            zip.extend_from_slice(content);
        }

        // ── Central directory ────────────────────────────────────────
        let cd_offset = zip.len() as u32;
        for (i, &(name, content)) in entries.iter().enumerate() {
            let crc = crc32_checksum(content);
            let size = content.len() as u32;
            let name_bytes = name.as_bytes();

            zip.extend_from_slice(&CD_SIGNATURE.to_le_bytes());
            zip.extend_from_slice(&20_u16.to_le_bytes());
            zip.extend_from_slice(&20_u16.to_le_bytes());
            zip.extend_from_slice(&0_u16.to_le_bytes());
            zip.extend_from_slice(&METHOD_STORED.to_le_bytes());
            zip.extend_from_slice(&0_u16.to_le_bytes());
            zip.extend_from_slice(&0_u16.to_le_bytes());
            zip.extend_from_slice(&crc.to_le_bytes());
            zip.extend_from_slice(&size.to_le_bytes());
            zip.extend_from_slice(&size.to_le_bytes());
            zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            zip.extend_from_slice(&0_u16.to_le_bytes());
            zip.extend_from_slice(&0_u16.to_le_bytes());
            zip.extend_from_slice(&0_u16.to_le_bytes());
            zip.extend_from_slice(&0_u16.to_le_bytes());
            zip.extend_from_slice(&0_u32.to_le_bytes());
            zip.extend_from_slice(&local_offsets[i].to_le_bytes());
            zip.extend_from_slice(name_bytes);
        }
        let cd_size = (zip.len() as u32) - cd_offset;

        // ── EOCD ─────────────────────────────────────────────────────
        zip.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());
        zip.extend_from_slice(&count.to_le_bytes());
        zip.extend_from_slice(&count.to_le_bytes());
        zip.extend_from_slice(&cd_size.to_le_bytes());
        zip.extend_from_slice(&cd_offset.to_le_bytes());
        zip.extend_from_slice(&0_u16.to_le_bytes());

        zip
    }

    // ── CRC-32 ───────────────────────────────────────────────────────────

    #[test]
    fn crc32_empty() {
        assert_eq!(crc32_checksum(b""), 0x0000_0000);
    }

    #[test]
    fn crc32_known_value() {
        // "123456789" has well-known CRC32 = 0xCBF43926.
        assert_eq!(crc32_checksum(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn zip_open_io_error() {
        let err = ZipReader::open(Path::new("/does/not/exist/ever/zip.zip")).unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
    }

    #[test]
    fn local_header_truncated() {
        let zip = build_stored_zip("test.txt", b"data");
        let reader = ZipReader::from_bytes(zip).expect("should parse");

        // Corrupt the local header offset to point near the end of the file
        let info = reader.get_entry("test.txt").unwrap();
        let mut corrupted_info = info.clone();
        corrupted_info.local_header_offset = (reader.data.len() - 10) as u64;

        let err = reader.read_entry_info(&corrupted_info).unwrap_err();
        assert!(matches!(err, Error::ZipFormat { .. }));
        if let Error::ZipFormat { msg } = err {
            assert!(msg.contains("is truncated"));
        }
    }

    #[test]
    fn local_header_bad_signature() {
        let mut zip = build_stored_zip("test.txt", b"data");
        // Corrupt the local header signature (first 4 bytes)
        zip[0] ^= 0xFF;

        let reader = ZipReader::from_bytes(zip).expect("should parse");
        let info = reader.get_entry("test.txt").unwrap();

        let err = reader.read_entry_info(info).unwrap_err();
        assert!(matches!(err, Error::ZipFormat { .. }));
        if let Error::ZipFormat { msg } = err {
            assert!(msg.contains("expected local header signature"));
        }
    }

    #[test]
    fn data_extends_past_eof() {
        let zip = build_stored_zip("test.txt", b"data");
        let reader = ZipReader::from_bytes(zip).expect("should parse");

        let info = reader.get_entry("test.txt").unwrap();
        let mut corrupted_info = info.clone();
        corrupted_info.compressed_size = (reader.data.len() + 10) as u64;

        let err = reader.read_entry_info(&corrupted_info).unwrap_err();
        assert!(matches!(err, Error::ZipFormat { .. }));
        if let Error::ZipFormat { msg } = err {
            assert!(msg.contains("data extends past end of archive"));
        }
    }

    #[test]
    fn unsupported_compression_method() {
        let zip = build_stored_zip("test.txt", b"data");
        let reader = ZipReader::from_bytes(zip).expect("should parse");

        let info = reader.get_entry("test.txt").unwrap();
        let mut corrupted_info = info.clone();
        corrupted_info.compression_method = 99; // 99 is unsupported

        let err = reader.read_entry_info(&corrupted_info).unwrap_err();
        assert!(matches!(err, Error::ZipFormat { .. }));
        if let Error::ZipFormat { msg } = err {
            assert!(msg.contains("unsupported compression method"));
        }
    }

    #[test]
    fn deflate_error() {
        let mut zip = build_deflated_zip("test.txt", b"data that will be compressed");
        // Corrupt the DEFLATE stream data
        // Local header ends at 30 + filename_len(8) = 38
        zip[38] ^= 0xFF;

        let reader = ZipReader::from_bytes(zip).expect("should parse");
        let info = reader.get_entry("test.txt").unwrap();

        let err = reader.read_entry_info(info).unwrap_err();
        assert!(matches!(err, Error::ZipFormat { .. }));
        if let Error::ZipFormat { msg } = err {
            assert!(msg.contains("failed to deflate entry"));
        }
    }

    #[test]
    fn table_driven_eocd_errors() {
        struct TestCase {
            name: &'static str,
            data: Vec<u8>,
            expected_msg: &'static str,
        }

        let cases = vec![
            TestCase {
                name: "central directory extends past end of file",
                data: {
                    let mut data: Vec<u8> = vec![0; 22]; // EOCD size
                    data[0..4].copy_from_slice(&EOCD_SIGNATURE.to_le_bytes());
                    // cd_size
                    data[12..16].copy_from_slice(&100u32.to_le_bytes());
                    // cd_offset
                    data[16..20].copy_from_slice(&0u32.to_le_bytes());
                    data
                },
                expected_msg: "central directory extends past end of file",
            },
            TestCase {
                name: "central directory entry truncated",
                data: {
                    let mut data: Vec<u8> = vec![0; 40]; // Enough for EOCD + some CD
                    // EOCD at offset 18
                    data[18..22].copy_from_slice(&EOCD_SIGNATURE.to_le_bytes());
                    data[28..30].copy_from_slice(&1u16.to_le_bytes()); // entry_count
                    data[30..34].copy_from_slice(&46u32.to_le_bytes()); // cd_size (min for 1 entry)
                    data[34..38].copy_from_slice(&0u32.to_le_bytes()); // cd_offset

                    // CD start
                    data[0..4].copy_from_slice(&CD_SIGNATURE.to_le_bytes());
                    // The CD entry is truncated because total size is 40, cd_offset = 0, cd_size = 46.
                    // Oh wait, if cd_size = 46, cd_offset = 0, then cd_offset + cd_size = 46 > data.len() (40).
                    // This hits "extends past end of file".

                    // Let's make data bigger: 50 bytes.
                    let mut data2: Vec<u8> = vec![0; 50];
                    data2[28..32].copy_from_slice(&EOCD_SIGNATURE.to_le_bytes()); // EOCD at 28
                    data2[38..40].copy_from_slice(&1u16.to_le_bytes()); // count
                    data2[40..44].copy_from_slice(&10u32.to_le_bytes()); // cd_size = 10, but count=1.
                    // cd_offset = 0.
                    // 0 + 10 <= 50. So it passes EOCD check.
                    // parse_central_directory expects 1 entry, and size is 10.
                    // cd_end = 10. pos = 0. pos + 46 (46) > cd_end (10).
                    data2[0..4].copy_from_slice(&CD_SIGNATURE.to_le_bytes());
                    data2
                },
                expected_msg: "central directory entry truncated",
            },
            TestCase {
                name: "bad cd signature",
                data: {
                    let mut data: Vec<u8> = vec![0; 100];
                    let eocd_pos = 78;
                    data[eocd_pos..eocd_pos + 4].copy_from_slice(&EOCD_SIGNATURE.to_le_bytes());
                    data[eocd_pos + 10..eocd_pos + 12].copy_from_slice(&1u16.to_le_bytes()); // count
                    data[eocd_pos + 12..eocd_pos + 16].copy_from_slice(&50u32.to_le_bytes()); // cd_size
                    data[eocd_pos + 16..eocd_pos + 20].copy_from_slice(&0u32.to_le_bytes()); // cd_offset
                    // Let's leave CD signature 0.
                    data
                },
                expected_msg: "expected central directory signature",
            },
            TestCase {
                name: "central directory entry filename truncated",
                data: {
                    let mut data: Vec<u8> = vec![0; 100];
                    let eocd_pos = 78;
                    data[eocd_pos..eocd_pos + 4].copy_from_slice(&EOCD_SIGNATURE.to_le_bytes());
                    data[eocd_pos + 10..eocd_pos + 12].copy_from_slice(&1u16.to_le_bytes()); // count
                    data[eocd_pos + 12..eocd_pos + 16].copy_from_slice(&50u32.to_le_bytes()); // cd_size
                    data[eocd_pos + 16..eocd_pos + 20].copy_from_slice(&0u32.to_le_bytes()); // cd_offset

                    data[0..4].copy_from_slice(&CD_SIGNATURE.to_le_bytes());
                    data[28..30].copy_from_slice(&10u16.to_le_bytes()); // filename_len = 10
                    // pos + 46 + 10 = 56. cd_end = 50. 56 > 50 -> trunc
                    data
                },
                expected_msg: "central directory entry filename truncated",
            },
        ];

        for case in cases {
            let res = ZipReader::from_bytes(case.data);
            assert!(res.is_err(), "Test case failed: {}", case.name);
            let err = res.unwrap_err();
            if let Error::ZipFormat { msg } = err {
                assert!(
                    msg.contains(case.expected_msg),
                    "Case '{}' expected msg containing '{}', got '{}'",
                    case.name,
                    case.expected_msg,
                    msg
                );
            } else {
                panic!(
                    "Case '{}' expected ZipFormat error, got {:?}",
                    case.name, err
                );
            }
        }
    }

    // ── ZipReader from bytes ─────────────────────────────────────────────

    #[test]
    fn stored_entry_round_trip() {
        let content = b"hello, zip world!";
        let zip = build_stored_zip("greeting.txt", content);
        let reader = ZipReader::from_bytes(zip).expect("should parse");
        assert_eq!(reader.entry_count(), 1);
        let data = reader.read_entry("greeting.txt").expect("should read");
        assert_eq!(data, content);
    }

    #[test]
    fn deflated_entry_round_trip() {
        let content = b"the quick brown fox jumps over the lazy dog, repeatedly.";
        let zip = build_deflated_zip("fox.txt", content);
        let reader = ZipReader::from_bytes(zip).expect("should parse");
        assert_eq!(reader.entry_count(), 1);
        let data = reader.read_entry("fox.txt").expect("should read");
        assert_eq!(data, content.as_slice());
    }

    #[test]
    fn multiple_entries() {
        let zip = build_multi_entry_zip(&[
            ("a.txt", b"alpha"),
            ("b.txt", b"bravo"),
            ("c.txt", b"charlie"),
        ]);
        let reader = ZipReader::from_bytes(zip).expect("should parse");
        assert_eq!(reader.entry_count(), 3);
        assert_eq!(reader.read_entry("a.txt").unwrap(), b"alpha".to_vec());
        assert_eq!(reader.read_entry("b.txt").unwrap(), b"bravo".to_vec());
        assert_eq!(reader.read_entry("c.txt").unwrap(), b"charlie".to_vec());
    }

    #[test]
    fn entry_not_found() {
        let zip = build_stored_zip("exists.txt", b"data");
        let reader = ZipReader::from_bytes(zip).expect("should parse");
        let err = reader.read_entry("missing.txt").unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[test]
    fn empty_archive() {
        let zip = build_multi_entry_zip(&[]);
        let reader = ZipReader::from_bytes(zip).expect("should parse");
        assert_eq!(reader.entry_count(), 0);
    }

    #[test]
    fn crc32_mismatch_detected() {
        let mut zip = build_stored_zip("data.bin", b"original");
        // Corrupt one byte of the stored content (first byte after local header).
        // Local header = 30 + filename_len(8) = 38; data starts at 38.
        zip[38] ^= 0xFF;
        let reader = ZipReader::from_bytes(zip).expect("index should parse");
        let err = reader.read_entry("data.bin").unwrap_err();
        assert!(matches!(err, Error::ZipCrc32 { .. }));
    }

    #[test]
    fn bad_eocd_signature() {
        let err = ZipReader::from_bytes(vec![0; 30]).unwrap_err();
        assert!(matches!(err, Error::ZipFormat { .. }));
    }

    #[test]
    fn file_too_small() {
        let err = ZipReader::from_bytes(vec![0; 10]).unwrap_err();
        assert!(matches!(err, Error::ZipFormat { .. }));
    }

    #[test]
    fn get_entry_returns_metadata() {
        let zip = build_stored_zip("test.txt", b"content");
        let reader = ZipReader::from_bytes(zip).expect("should parse");
        let info = reader.get_entry("test.txt").expect("entry should exist");
        assert_eq!(info.name, "test.txt");
        assert_eq!(info.compression_method, METHOD_STORED);
        assert_eq!(info.uncompressed_size, 7);
    }

    #[test]
    fn entry_names_lists_all() {
        let zip = build_multi_entry_zip(&[("x.txt", b"x"), ("y.txt", b"y")]);
        let reader = ZipReader::from_bytes(zip).expect("should parse");
        let mut names: Vec<&str> = reader.entry_names().collect();
        names.sort_unstable();
        assert_eq!(names, vec!["x.txt", "y.txt"]);
    }

    // ── ZipLoader (ClassLoader) ──────────────────────────────────────────

    #[test]
    fn zip_loader_finds_class() {
        // Fake a minimal class file with magic bytes.
        let fake_class = [0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 65];
        let zip = build_stored_zip("com/example/Main.class", &fake_class);

        let tmp = std::env::temp_dir().join("duke_test_zip_loader.jar");
        std::fs::write(&tmp, &zip).unwrap();
        let loader = ZipLoader::open(&tmp).expect("should open");
        let bytes = loader
            .find_class("com/example/Main")
            .expect("should find class");
        assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn zip_loader_not_found() {
        let zip = build_stored_zip("Other.class", b"data");
        let tmp = std::env::temp_dir().join("duke_test_zip_notfound.jar");
        std::fs::write(&tmp, &zip).unwrap();
        let loader = ZipLoader::open(&tmp).expect("should open");
        let err = loader.find_class("Missing").unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn zip_loader_read_entry_other_error() {
        let mut zip = build_stored_zip("Bad.class", b"data");
        zip[0] ^= 0xFF; // Corrupt local header signature

        let tmp = std::env::temp_dir().join("duke_test_zip_bad.jar");
        std::fs::write(&tmp, &zip).unwrap();
        let loader = ZipLoader::open(&tmp).expect("should open");
        let err = loader.find_class("Bad").unwrap_err();
        assert!(matches!(err, Error::ZipFormat { .. }));
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn zip_loader_read_entry_other_error_boot_inf() {
        let mut zip = build_stored_zip("BOOT-INF/classes/Bad.class", b"data");
        zip[0] ^= 0xFF; // Corrupt local header signature

        let tmp = std::env::temp_dir().join("duke_test_zip_bad_boot_inf.jar");
        std::fs::write(&tmp, &zip).unwrap();
        let loader = ZipLoader::open(&tmp).expect("should open");
        let err = loader.find_class("Bad").unwrap_err();
        assert!(matches!(err, Error::ZipFormat { .. }));
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn zip_loader_try_nested_read_entry_error() {
        let mut nested_jar = build_stored_zip("Bad.class", b"data");
        nested_jar[0] ^= 0xFF; // Corrupt local header signature
        let outer_zip = build_multi_entry_zip(&[("BOOT-INF/lib/dependency.jar", &nested_jar)]);
        let tmp = std::env::temp_dir().join("duke_test_zip_nested_err_legacy.jar");
        std::fs::write(&tmp, &outer_zip).unwrap();

        let loader = ZipLoader::open(&tmp).expect("should open outer zip");
        // The inner ZipReader will fail to read "Bad.class" during find_class
        // and should bubble the error out.
        let err = loader.find_class("Bad").unwrap_err();
        assert!(matches!(err, super::Error::ZipFormat { .. }));
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn zip_loader_read_entry_other_error_nested() {
        let mut nested_jar = build_stored_zip("Bad.class", b"data");
        nested_jar[0] ^= 0xFF; // Corrupt local header signature
        let outer_zip = build_multi_entry_zip(&[("BOOT-INF/lib/dependency.jar", &nested_jar)]);

        let tmp = std::env::temp_dir().join("duke_test_zip_bad_nested.jar");
        std::fs::write(&tmp, &outer_zip).unwrap();
        let loader = ZipLoader::open(&tmp).expect("should open");
        let err = loader.find_class("Bad").unwrap_err();
        assert!(matches!(err, Error::ZipFormat { .. }));
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn zip_loader_finds_class_in_boot_inf_classes() {
        let fake_class = [0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 65];
        let zip = build_multi_entry_zip(&[("BOOT-INF/classes/com/example/App.class", &fake_class)]);

        let tmp = std::env::temp_dir().join("duke_test_boot_inf_classes.jar");
        std::fs::write(&tmp, &zip).unwrap();
        let loader = ZipLoader::open(&tmp).expect("should open");
        let bytes = loader
            .find_class("com/example/App")
            .expect("should find class in BOOT-INF/classes");
        assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn zip_loader_finds_class_in_nested_boot_inf_lib_jar() {
        let fake_class = [0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 65];
        let nested_jar = build_multi_entry_zip(&[("com/example/Dependency.class", &fake_class)]);
        let outer_zip = build_multi_entry_zip(&[("BOOT-INF/lib/dependency.jar", &nested_jar)]);

        let tmp = std::env::temp_dir().join("duke_test_boot_inf_lib.jar");
        std::fs::write(&tmp, &outer_zip).unwrap();
        let loader = ZipLoader::open(&tmp).expect("should open");
        let bytes = loader
            .find_class("com/example/Dependency")
            .expect("should find class in nested BOOT-INF/lib jar");
        assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn zip_loader_prefers_boot_inf_classes_before_nested_libs() {
        let app_class = [0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 65];
        let nested_class = [0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 66];
        let nested_jar = build_multi_entry_zip(&[("com/example/App.class", &nested_class)]);
        let outer_zip = build_multi_entry_zip(&[
            ("BOOT-INF/classes/com/example/App.class", &app_class),
            ("BOOT-INF/lib/dependency.jar", &nested_jar),
        ]);

        let tmp = std::env::temp_dir().join("duke_test_boot_inf_precedence.jar");
        std::fs::write(&tmp, &outer_zip).unwrap();
        let loader = ZipLoader::open(&tmp).expect("should open");
        let bytes = loader
            .find_class("com/example/App")
            .expect("should prefer BOOT-INF/classes");
        assert_eq!(bytes, app_class);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_zip_missing_eocd() {
        let zip_bytes = vec![0u8; 100];
        let err = ZipReader::from_bytes(zip_bytes).unwrap_err();
        assert!(
            matches!(err, Error::ZipFormat { ref msg } if msg == "could not find end-of-central-directory record")
        );
    }

    #[test]
    fn test_zip_cd_extends_past_eof() {
        let mut zip_bytes = build_stored_zip("test.txt", b"hello world");
        let len = zip_bytes.len();
        let eocd_pos = len - 22;
        zip_bytes[eocd_pos + 12..eocd_pos + 16].copy_from_slice(&u32::MAX.to_le_bytes());
        let err = ZipReader::from_bytes(zip_bytes).unwrap_err();
        assert!(
            matches!(err, Error::ZipFormat { ref msg } if msg == "central directory extends past end of file")
        );
    }

    #[test]
    fn test_cd_entry_filename_truncated() {
        let mut zip_data = Vec::new();
        let central_dir_offset = 0;
        let cd_signature: u32 = 0x0201_4b50;
        zip_data.extend_from_slice(&cd_signature.to_le_bytes()); // CD signature
        zip_data.extend_from_slice(&[0; 24]);
        let name = b"abcde";
        zip_data.extend_from_slice(&(10_u16).to_le_bytes()); // filename len
        zip_data.extend_from_slice(&0_u16.to_le_bytes()); // extra len
        zip_data.extend_from_slice(&0_u16.to_le_bytes()); // comment len
        zip_data.extend_from_slice(&[0; 8]);
        zip_data.extend_from_slice(&0_u32.to_le_bytes()); // local header offset
        zip_data.extend_from_slice(name); // filename (truncated)

        let central_dir_size = (zip_data.len() as u32) - central_dir_offset;
        let eocd_signature: u32 = 0x0605_4b50;
        zip_data.extend_from_slice(&eocd_signature.to_le_bytes());
        zip_data.extend_from_slice(&[0; 4]);
        zip_data.extend_from_slice(&1_u16.to_le_bytes());
        zip_data.extend_from_slice(&1_u16.to_le_bytes());
        zip_data.extend_from_slice(&central_dir_size.to_le_bytes());
        zip_data.extend_from_slice(&central_dir_offset.to_le_bytes());
        zip_data.extend_from_slice(&0_u16.to_le_bytes());

        let err = ZipReader::from_bytes(zip_data).unwrap_err();
        assert!(
            matches!(err, Error::ZipFormat { ref msg } if msg == "central directory entry filename truncated")
        );
    }

    #[test]
    fn test_cd_entry_truncated() {
        let mut zip_data = Vec::new();
        let central_dir_offset = 0;
        let cd_signature: u32 = 0x0201_4b50;
        zip_data.extend_from_slice(&cd_signature.to_le_bytes());
        zip_data.extend_from_slice(&[0; 10]); // Truncated CD entry
        let central_dir_size = (zip_data.len() as u32) - central_dir_offset;

        let eocd_signature: u32 = 0x0605_4b50;
        zip_data.extend_from_slice(&eocd_signature.to_le_bytes());
        zip_data.extend_from_slice(&[0; 4]);
        zip_data.extend_from_slice(&1_u16.to_le_bytes());
        zip_data.extend_from_slice(&1_u16.to_le_bytes());
        zip_data.extend_from_slice(&central_dir_size.to_le_bytes());
        zip_data.extend_from_slice(&central_dir_offset.to_le_bytes());
        zip_data.extend_from_slice(&0_u16.to_le_bytes());

        let err = ZipReader::from_bytes(zip_data).unwrap_err();
        assert!(
            matches!(err, Error::ZipFormat { ref msg } if msg == "central directory entry truncated")
        );
    }

    #[test]
    fn test_zip_uncompressed_limit_exceeded() {
        // Build a minimal zip and corrupt the uncompressed size.
        let zip = build_deflated_zip("huge.txt", b"small data");
        let reader = ZipReader::from_bytes(zip).expect("should parse");

        let info = reader.get_entry("huge.txt").unwrap();
        let mut corrupted_info = info.clone();

        // Corrupt uncompressed size to be greater than 256MB
        corrupted_info.uncompressed_size = (1024 * 1024 * 257) as u64;

        let err = reader.read_entry_info(&corrupted_info).unwrap_err();
        assert!(matches!(err, Error::ZipFormat { ref msg } if msg.contains("exceeds limit")));
    }

    #[test]
    fn test_zip_loader_try_nested_read_entry_error() {
        let mut nested_jar = build_stored_zip("Bad.class", b"data");
        nested_jar[0] ^= 0xFF;
        let outer_zip = build_multi_entry_zip(&[("BOOT-INF/lib/dependency.jar", &nested_jar)]);
        let tmp = std::env::temp_dir().join("duke_test_zip_nested_err.jar");
        std::fs::write(&tmp, &outer_zip).unwrap();
        let loader = ZipLoader::open(&tmp).expect("should open outer zip");
        let err = loader.find_class("Bad").unwrap_err();
        assert!(matches!(err, super::Error::ZipFormat { .. }));
        std::fs::remove_file(&tmp).ok();
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashMap;

    proptest! {
        #[test]
        fn fuzz_zip_reader_read_entry_info(
            uncompressed_size in any::<u64>()
        ) {
            let info = ZipEntryInfo {
                name: "fuzz.txt".to_string(),
                compression_method: METHOD_DEFLATED,
                crc32: 0,
                compressed_size: 0,
                uncompressed_size,
                local_header_offset: 0,
            };

            // Let's make a mock local header signature + empty filename/extra + empty data
            let mut data = vec![0; 30];
            data[0..4].copy_from_slice(&LOCAL_SIGNATURE.to_le_bytes());

            let reader = ZipReader {
                data,
                index: HashMap::with_capacity(0),
            };

            let _ = reader.read_entry_info(&info);
        }
    }
}
