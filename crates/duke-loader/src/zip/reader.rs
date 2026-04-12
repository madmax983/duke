use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use crate::{LoadError, LoadResult};

// ───────────────────────────────────────────────────────────────────────────
// ZIP format constants
// ───────────────────────────────────────────────────────────────────────────

pub(crate) const EOCD_SIGNATURE: u32 = 0x0605_4b50; // PK
pub(crate) const CD_SIGNATURE: u32 = 0x0201_4b50; // PK
pub(crate) const LOCAL_SIGNATURE: u32 = 0x0403_4b50; // PK
pub(crate) const METHOD_STORED: u16 = 0;
pub(crate) const METHOD_DEFLATED: u16 = 8;

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

pub(crate) fn crc32_checksum(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        let idx = ((crc ^ u32::from(byte)) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32_TABLE[idx];
    }
    !crc
}

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
    pub(crate) local_header_offset: u64,
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
    pub(crate) data: Vec<u8>,
    pub(crate) index: HashMap<String, ZipEntryInfo>,
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
        let offset = info.local_header_offset as usize;

        // Validate local file header signature.
        if offset + 30 > self.data.len() {
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
        let data_start = offset + 30 + filename_len + extra_len;
        let compressed_size = info.compressed_size as usize;

        if data_start + compressed_size > self.data.len() {
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

    #[cfg(test)]
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }
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
    let safe_capacity = expected_count.min(cd_size / 46);
    let mut index = HashMap::with_capacity(safe_capacity);
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
