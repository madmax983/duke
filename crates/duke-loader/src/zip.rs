//! Read-only ZIP/JAR archive support.
//!
//! Implements index-on-open, lazy decompression — the same pattern as
//! [`super::jimage::JImageReader`].  Used both by the internal classloader
//! (`ZipLoader`) and by the Java-space `ZipFile` native bridges.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use crate::{ClassLoader, LoadError, LoadResult};

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
    /// # Errors
    /// Returns [`LoadError::Io`] on read failure, or [`LoadError::ZipFormat`]
    /// if the file is not a valid ZIP archive.
    pub fn open(path: &Path) -> LoadResult<Self> {
        let data = std::fs::read(path).map_err(|source| LoadError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_bytes(data)
    }

    /// Build a `ZipReader` from raw bytes (useful for tests).
    ///
    /// # Errors
    /// Returns [`LoadError::ZipFormat`] if the data is not a valid ZIP archive.
    pub fn from_bytes(data: Vec<u8>) -> LoadResult<Self> {
        let eocd_pos = find_eocd(&data)?;
        let index = parse_eocd_and_central_directory(&data, eocd_pos)?;
        Ok(Self { data, index })
    }

    /// Look up an entry by name.  O(1).
    #[must_use]
    pub fn get_entry(&self, name: &str) -> Option<&ZipEntryInfo> {
        self.index.get(name)
    }

    /// Decompress and return the bytes for the named entry.
    ///
    /// Validates the CRC-32 after decompression.
    ///
    /// # Errors
    /// Returns [`LoadError::NotFound`] if the entry doesn't exist,
    /// [`LoadError::ZipFormat`] on decompression failure, or
    /// [`LoadError::ZipCrc32`] on checksum mismatch.
    pub fn read_entry(&self, name: &str) -> LoadResult<Vec<u8>> {
        let info = self.index.get(name).ok_or_else(|| LoadError::NotFound {
            name: name.to_string(),
        })?;
        self.read_entry_info(info)
    }

    /// Read entry bytes given a pre-looked-up `ZipEntryInfo`.
    ///
    /// # Errors
    /// Returns [`LoadError::ZipFormat`] on decompression or format errors,
    /// or [`LoadError::ZipCrc32`] on checksum mismatch.
    #[allow(clippy::cast_possible_truncation)]
    pub fn read_entry_info(&self, info: &ZipEntryInfo) -> LoadResult<Vec<u8>> {
        let offset = usize::try_from(info.local_header_offset).unwrap_or(usize::MAX);

        // Validate local file header signature.
        if offset.saturating_add(30) > self.data.len() {
            return Err(LoadError::ZipFormat {
                msg: format!("local header at offset {offset} is truncated"),
            });
        }
        let sig = read_u32_le(&self.data, offset);
        if sig != LOCAL_SIGNATURE {
            return Err(LoadError::ZipFormat {
                msg: format!("expected local header signature at offset {offset}, got {sig:#010x}"),
            });
        }

        // Read local header's own filename_len and extra_len to find data start.
        let filename_len = read_u16_le(&self.data, offset + 26) as usize;
        let extra_len = read_u16_le(&self.data, offset + 28) as usize;
        let data_start = offset.saturating_add(30).saturating_add(filename_len).saturating_add(extra_len);
        let compressed_size = usize::try_from(info.compressed_size).unwrap_or(usize::MAX);

        if data_start.saturating_add(compressed_size) > self.data.len() {
            return Err(LoadError::ZipFormat {
                msg: format!("entry '{}' data extends past end of archive", info.name),
            });
        }

        let compressed = &self.data[data_start..data_start + compressed_size];

        let decompressed = match info.compression_method {
            METHOD_STORED => compressed.to_vec(),
            METHOD_DEFLATED => {
                let mut decoder = flate2::read::DeflateDecoder::new(compressed);
                let cap = info.uncompressed_size as usize;
                let mut buf = Vec::with_capacity(cap.min(1024 * 1024 * 32));
                decoder
                    .read_to_end(&mut buf)
                    .map_err(|_| LoadError::ZipFormat {
                        msg: format!("failed to deflate entry '{}'", info.name),
                    })?;
                buf
            }
            other => {
                return Err(LoadError::ZipFormat {
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
            return Err(LoadError::ZipCrc32 {
                name: info.name.clone(),
                expected: info.crc32,
                actual: actual_crc,
            });
        }

        Ok(decompressed)
    }

    /// Number of entries in the archive.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.index.len()
    }

    /// Iterate over all entry names.
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

    fn from_reader(reader: ZipReader) -> LoadResult<Self> {
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
            Ok(bytes) => return Ok(bytes),
            Err(LoadError::NotFound { .. }) => {}
            Err(other) => return Err(other),
        }

        // Try BOOT-INF path: BOOT-INF/classes/{name}.class
        entry_name.clear();
        entry_name.push_str("BOOT-INF/classes/");
        entry_name.push_str(name);
        entry_name.push_str(".class");
        match self.reader.read_entry(&entry_name) {
            Ok(bytes) => return Ok(bytes),
            Err(LoadError::NotFound { .. }) => {}
            Err(other) => return Err(other),
        }

        for nested_lib in &self.nested_libs {
            match nested_lib.find_class(name) {
                Ok(bytes) => return Ok(bytes),
                Err(LoadError::NotFound { .. }) => {}
                Err(other) => return Err(other),
            }
        }
        Err(LoadError::NotFound {
            name: name.to_string(),
        })
    }
}

fn nested_boot_inf_lib_loaders(reader: &ZipReader) -> LoadResult<Vec<ZipLoader>> {
    let mut nested_entry_names: Vec<String> = reader
        .entry_names()
        .filter(|name| is_nested_boot_inf_lib_archive(name))
        .map(str::to_owned)
        .collect();
    nested_entry_names.sort_unstable();

    let mut nested_libs = Vec::with_capacity(nested_entry_names.len());
    for entry_name in nested_entry_names {
        let nested_bytes = reader.read_entry(&entry_name)?;
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
fn find_eocd(data: &[u8]) -> LoadResult<usize> {
    if data.len() < EOCD_MIN_SIZE {
        return Err(LoadError::ZipFormat {
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
    Err(LoadError::ZipFormat {
        msg: "could not find end-of-central-directory record".to_string(),
    })
}

/// Parse the EOCD record and then the central directory it points to.
fn parse_eocd_and_central_directory(
    data: &[u8],
    eocd_pos: usize,
) -> LoadResult<HashMap<String, ZipEntryInfo>> {
    // EOCD layout (22 bytes minimum):
    //  0: signature (4)
    //  4: disk number (2)
    //  6: disk where CD starts (2)
    //  8: number of CD entries on this disk (2)
    // 10: total number of CD entries (2)
    // 12: size of CD (4)
    // 16: offset of start of CD (4)
    // 20: comment length (2)
    let entry_count = read_u16_le(data, eocd_pos + 10) as usize;
    let cd_size = read_u32_le(data, eocd_pos + 12) as usize;
    let cd_offset = read_u32_le(data, eocd_pos + 16) as usize;

    if cd_offset + cd_size > data.len() {
        return Err(LoadError::ZipFormat {
            msg: "central directory extends past end of file".to_string(),
        });
    }

    parse_central_directory(data, cd_offset, cd_size, entry_count)
}

/// Parse central directory entries into a `HashMap`.
fn parse_central_directory(
    data: &[u8],
    cd_offset: usize,
    cd_size: usize,
    expected_count: usize,
) -> LoadResult<HashMap<String, ZipEntryInfo>> {
    let mut index = HashMap::with_capacity(expected_count);
    let cd_end = cd_offset + cd_size;
    let mut pos = cd_offset;

    for _ in 0..expected_count {
        // Each central directory header is at least 46 bytes.
        if pos + 46 > cd_end {
            return Err(LoadError::ZipFormat {
                msg: "central directory entry truncated".to_string(),
            });
        }
        let sig = read_u32_le(data, pos);
        if sig != CD_SIGNATURE {
            return Err(LoadError::ZipFormat {
                msg: format!(
                    "expected central directory signature at offset {pos}, got {sig:#010x}"
                ),
            });
        }

        let compression_method = read_u16_le(data, pos + 10);
        let crc32 = read_u32_le(data, pos + 16);
        let compressed_size = u64::from(read_u32_le(data, pos + 20));
        let uncompressed_size = u64::from(read_u32_le(data, pos + 24));
        let filename_len = read_u16_le(data, pos + 28) as usize;
        let extra_len = read_u16_le(data, pos + 30) as usize;
        let comment_len = read_u16_le(data, pos + 32) as usize;
        let local_header_offset = u64::from(read_u32_le(data, pos + 42));

        let name_start = pos + 46;
        if name_start + filename_len > cd_end {
            return Err(LoadError::ZipFormat {
                msg: "central directory entry filename truncated".to_string(),
            });
        }

        let name =
            String::from_utf8_lossy(&data[name_start..name_start + filename_len]).into_owned();

        index.insert(
            name.clone(),
            ZipEntryInfo {
                name,
                compression_method,
                crc32,
                compressed_size,
                uncompressed_size,
                local_header_offset,
            },
        );

        pos = name_start + filename_len + extra_len + comment_len;
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

    // ── Helpers: build minimal valid ZIPs in memory ──────────────────────

    /// Build a minimal ZIP archive containing one STORED entry.
    fn build_stored_zip(name: &str, content: &[u8]) -> Vec<u8> {
        let crc = crc32_checksum(content);
        let size = content.len() as u32;
        let name_bytes = name.as_bytes();

        let mut zip = Vec::new();

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

        let mut zip = Vec::new();

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
        let mut local_offsets = Vec::new();

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
        assert!(matches!(err, LoadError::Io { .. }));
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
        assert!(matches!(err, LoadError::ZipFormat { .. }));
        if let LoadError::ZipFormat { msg } = err {
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
        assert!(matches!(err, LoadError::ZipFormat { .. }));
        if let LoadError::ZipFormat { msg } = err {
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
        assert!(matches!(err, LoadError::ZipFormat { .. }));
        if let LoadError::ZipFormat { msg } = err {
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
        assert!(matches!(err, LoadError::ZipFormat { .. }));
        if let LoadError::ZipFormat { msg } = err {
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
        assert!(matches!(err, LoadError::ZipFormat { .. }));
        if let LoadError::ZipFormat { msg } = err {
            assert!(msg.contains("failed to deflate entry"));
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
        assert!(matches!(err, LoadError::NotFound { .. }));
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
        assert!(matches!(err, LoadError::ZipCrc32 { .. }));
    }

    #[test]
    fn bad_eocd_signature() {
        let err = ZipReader::from_bytes(vec![0; 30]).unwrap_err();
        assert!(matches!(err, LoadError::ZipFormat { .. }));
    }

    #[test]
    fn file_too_small() {
        let err = ZipReader::from_bytes(vec![0; 10]).unwrap_err();
        assert!(matches!(err, LoadError::ZipFormat { .. }));
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
        assert!(matches!(err, LoadError::NotFound { .. }));
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
        assert!(matches!(err, LoadError::ZipFormat { .. }));
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
        assert!(matches!(err, LoadError::ZipFormat { .. }));
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
        assert!(matches!(err, LoadError::ZipFormat { .. }));
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
                index: HashMap::new(),
            };

            let _ = reader.read_entry_info(&info);
        }
    }
}
