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
    /// Open and parse a jimage file, building the resource index.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError::Io`] if the file cannot be read, or
    /// [`LoadError::JImageFormat`] if the file is not a valid jimage.
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

        let index = build_index(&data, locations_offset, ls, strings_offset);

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
}

// ---------------------------------------------------------------------------
// Location index builder
// ---------------------------------------------------------------------------

fn build_index(
    data: &[u8],
    locs_offset: usize,
    locs_size: usize,
    str_offset: usize,
) -> HashMap<String, ResourceInfo> {
    let mut index = HashMap::new();
    let mut pos = locs_offset;
    let locs_end = locs_offset + locs_size;

    while pos < locs_end {
        // Decode all attributes for this location entry
        let mut module: u64 = 0;
        let mut parent: u64 = 0;
        let mut base: u64 = 0;
        let mut extension: u64 = 0;
        let mut offset: u64 = 0;
        let mut compressed: u64 = 0;
        let mut uncompressed: u64 = 0;

        loop {
            if pos >= locs_end {
                break;
            }
            let hdr = data[pos];
            pos += 1;

            let kind = hdr >> 3;
            let len = usize::from((hdr & 7) + 1);

            if kind == ATTR_END {
                break;
            }
            if pos + len > data.len() {
                break;
            }

            let val = read_be_u64(&data[pos..pos + len]);
            pos += len;

            match kind {
                ATTR_MODULE => module = val,
                ATTR_PARENT => parent = val,
                ATTR_BASE => base = val,
                ATTR_EXTENSION => extension = val,
                ATTR_OFFSET => offset = val,
                ATTR_COMPRESSED => compressed = val,
                ATTR_UNCOMPRESSED => uncompressed = val,
                _ => {} // future attribute kinds — skip
            }
        }

        // Skip empty/padding entries (no base name or no data)
        if base == 0 || uncompressed == 0 {
            continue;
        }

        let mod_str = read_str(data, str_offset, module);
        let par_str = read_str(data, str_offset, parent);
        let base_str = read_str(data, str_offset, base);
        let ext_str = read_str(data, str_offset, extension);

        let path = build_jimage_path(mod_str, par_str, base_str, ext_str);
        if !path.is_empty() {
            index.insert(
                path,
                ResourceInfo {
                    offset,
                    compressed,
                    uncompressed,
                },
            );
        }
    }

    index
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a jimage path: `"/module/parent/base.extension"`.
///
/// If `parent` is empty: `"/module/base.extension"`.
/// If `extension` is empty: `"/module/parent/base"`.
fn build_jimage_path(module: &str, parent: &str, base: &str, extension: &str) -> String {
    if module.is_empty() || base.is_empty() {
        return String::new();
    }
    let mut path = format!("/{module}/");
    if !parent.is_empty() {
        path.push_str(parent);
        path.push('/');
    }
    path.push_str(base);
    if !extension.is_empty() {
        path.push('.');
        path.push_str(extension);
    }
    path
}

/// Split `"java/lang/Object"` into `("java/lang", "Object")`.
/// For a top-level name `"Foo"` returns `("", "Foo")`.
fn split_class_name(name: &str) -> (&str, &str) {
    name.rfind('/')
        .map_or(("", name), |pos| (&name[..pos], &name[pos + 1..]))
}

/// Read a null-terminated UTF-8 string from the string table.
fn read_str(data: &[u8], str_offset: usize, idx: u64) -> &str {
    let Ok(idx_usize) = usize::try_from(idx) else {
        return "";
    };
    let start = str_offset + idx_usize;
    if start >= data.len() {
        return "";
    }
    let end = data[start..]
        .iter()
        .position(|&b| b == 0)
        .map_or(data.len(), |p| start + p);
    std::str::from_utf8(&data[start..end]).unwrap_or("")
}

/// Read up to 8 bytes as a big-endian u64.
fn read_be_u64(bytes: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    let n = bytes.len().min(8);
    buf[8 - n..].copy_from_slice(&bytes[..n]);
    u64::from_be_bytes(buf)
}

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().expect("4 bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Encode a single jimage location attribute: header byte + big-endian value.
    fn attr(kind: u8, val: u64) -> Vec<u8> {
        // Minimum bytes needed to represent val
        let bytes_needed = (if val == 0 {
            1
        } else {
            (64 - val.leading_zeros() as usize).div_ceil(8)
        })
        .clamp(1, 8);
        let data = &val.to_be_bytes()[8 - bytes_needed..];
        let mut v = vec![(kind << 3) | (u8::try_from(bytes_needed).unwrap_or(8) - 1)];
        v.extend_from_slice(data);
        v
    }

    fn attr_end() -> u8 {
        0x00
    }

    // -----------------------------------------------------------------------
    // parse_header (lines 239: < → == and < → <=)
    // Kills: rejecting data with exactly HEADER_SIZE bytes
    // -----------------------------------------------------------------------

    #[test]
    fn parse_header_exactly_28_bytes_ok() {
        // Exactly HEADER_SIZE=28 bytes with valid magic + version.
        // `< → ==` mutant rejects data.len() == 28; `< → <=` does the same.
        let mut data = vec![0u8; 28];
        data[0..4].copy_from_slice(&JIMAGE_MAGIC.to_le_bytes());
        data[4..8].copy_from_slice(&JIMAGE_VERSION.to_le_bytes());
        assert!(
            parse_header(&data).is_ok(),
            "exactly 28 bytes should pass size check"
        );
    }

    // -----------------------------------------------------------------------
    // build_index basic — general loop correctness
    // -----------------------------------------------------------------------

    /// Construct a test data array: string table followed by location entries.
    /// Returns (data, `locs_offset`, `locs_size`, `str_offset`).
    fn make_build_index_data(locs: &[u8]) -> (Vec<u8>, usize, usize, usize) {
        // String table: "\0mod\0Foo\0"
        //   idx=0: '' (empty)
        //   idx=1: "mod"
        //   idx=5: "Foo"
        let strings: &[u8] = b"\x00mod\x00Foo\x00";
        let str_offset = 0usize;
        let locs_offset = strings.len();
        let mut data = strings.to_vec();
        data.extend_from_slice(locs);
        (data, locs_offset, locs.len(), str_offset)
    }

    #[test]
    fn build_index_basic_entry() {
        // MODULE=1("mod"), BASE=5("Foo"), UNCOMPRESSED=42, OFFSET=7, END
        let mut locs: Vec<u8> = Vec::new();
        locs.extend(attr(ATTR_MODULE, 1));
        locs.extend(attr(ATTR_BASE, 5));
        locs.extend(attr(ATTR_UNCOMPRESSED, 42));
        locs.extend(attr(ATTR_OFFSET, 7));
        locs.push(attr_end());

        let (data, locs_offset, locs_size, str_offset) = make_build_index_data(&locs);
        let index = build_index(&data, locs_offset, locs_size, str_offset);

        assert_eq!(index.len(), 1);
        let info = index.get("/mod/Foo").expect("expected /mod/Foo in index");
        assert_eq!(info.uncompressed, 42);
        assert_eq!(info.offset, 7);
        assert_eq!(info.compressed, 0);
    }

    // -----------------------------------------------------------------------
    // build_index: ATTR_COMPRESSED stored (line 317: delete match arm)
    // -----------------------------------------------------------------------

    #[test]
    fn build_index_stores_compressed_field() {
        // ATTR_COMPRESSED=5 should be stored; deleting the match arm leaves it as 0.
        let mut locs: Vec<u8> = Vec::new();
        locs.extend(attr(ATTR_MODULE, 1));
        locs.extend(attr(ATTR_BASE, 5));
        locs.extend(attr(ATTR_UNCOMPRESSED, 10));
        locs.extend(attr(ATTR_COMPRESSED, 5));
        locs.push(attr_end());

        let (data, locs_offset, locs_size, str_offset) = make_build_index_data(&locs);
        let index = build_index(&data, locs_offset, locs_size, str_offset);

        let info = index.get("/mod/Foo").expect("expected /mod/Foo in index");
        assert_eq!(info.compressed, 5, "compressed attribute must be stored");
    }

    // -----------------------------------------------------------------------
    // build_index: skip when uncompressed==0 (line 324: || → &&)
    // -----------------------------------------------------------------------

    #[test]
    fn build_index_skips_zero_uncompressed() {
        // base != 0 but uncompressed == 0 → entry skipped.
        // Mutant `&&` would NOT skip (only skips when BOTH zero).
        let mut locs: Vec<u8> = Vec::new();
        locs.extend(attr(ATTR_MODULE, 1));
        locs.extend(attr(ATTR_BASE, 5));
        locs.extend(attr(ATTR_UNCOMPRESSED, 0));
        locs.push(attr_end());

        let (data, locs_offset, locs_size, str_offset) = make_build_index_data(&locs);
        let index = build_index(&data, locs_offset, locs_size, str_offset);
        assert!(
            index.is_empty(),
            "entry with uncompressed=0 should be skipped"
        );
    }

    // -----------------------------------------------------------------------
    // build_index: bounds check at line 304 (> → ==, > → >=, + → -, + → *)
    // Two scenarios: exact fit (kills >=) and truncated (kills -, *, ==)
    // -----------------------------------------------------------------------

    #[test]
    fn build_index_exact_fit_attr_is_read() {
        // The UNCOMPRESSED attribute's last byte is exactly at data.len().
        // With `> → >=` mutant: pos+len >= data.len() → breaks → value not read → entry skipped.
        // With `> → ==` mutant: same.
        //
        // Layout: strings at [0..9], locs at [9..15] (no END byte — outer loop hits locs_end).
        //   str_offset=0, locs_offset=9, locs_size=6
        //   strings = b"\0mod\0Foo\0"  (9 bytes)
        //   locs    = [MODULE(idx=1), BASE(idx=5), UNCOMPRESSED(1)]  (no END)
        //   data.len() = 15

        let strings: &[u8] = b"\x00mod\x00Foo\x00"; // 9 bytes
        let mut locs: Vec<u8> = Vec::new();
        locs.extend(attr(ATTR_MODULE, 1)); // 2 bytes
        locs.extend(attr(ATTR_BASE, 5)); // 2 bytes
        locs.extend(attr(ATTR_UNCOMPRESSED, 1)); // 2 bytes  → locs = 6 bytes

        let mut data = strings.to_vec();
        data.extend_from_slice(&locs);
        // data.len() = 9 + 6 = 15; pos+len for last attr = (9+4+1) + 1 = 15 == data.len()
        let (locs_offset, locs_size, str_offset) = (9, 6, 0);

        let index = build_index(&data, locs_offset, locs_size, str_offset);
        let info = index
            .get("/mod/Foo")
            .expect("exact-fit attribute must be read into index");
        assert_eq!(info.uncompressed, 1);
    }

    #[test]
    fn build_index_truncated_attr_skips_safely() {
        // Attribute header claims len=4, but only 0 data bytes follow → pos+len > data.len().
        // Original: breaks safely, entry not added.
        // `+ → -` mutant: 3-4 wraps → huge number > data.len() → ALSO breaks (safe, same).
        // BUT: for 1-byte attrs with data at the very last byte, `+ → -` would try to read.
        // Use len=4, short data → triggers OOB path.
        //
        // locs = [MODULE(1), BASE(5,truncated header only — claims len=4 with no data)]
        //   After MODULE (2 bytes), pos is at locs_offset+2.
        //   Next: header 0x1B = (ATTR_BASE<<3)|(4-1) = 0x18|3 = 0x1B → kind=3, len=4.
        //   pos after hdr = locs_offset+3. pos+len = locs_offset+7 > data.len() → break.

        let strings: &[u8] = b"\x00mod\x00Foo\x00"; // 9 bytes
        let locs: Vec<u8> = vec![
            0x08, 0x01, // ATTR_MODULE (kind=1, len=1) = 1
            0x1B, // ATTR_BASE (kind=3, len=4) header only — no data bytes follow
        ];
        let mut data = strings.to_vec();
        data.extend_from_slice(&locs);
        // data.len() = 12; after BASE hdr at pos=locs_offset+2=11, pos=12, pos+len=16 > 12 → break
        let (locs_offset, locs_size, str_offset) = (9, locs.len(), 0);

        // Must not panic, and entry must not be indexed (BASE never set → path empty)
        let index = build_index(&data, locs_offset, locs_size, str_offset);
        assert!(
            index.is_empty(),
            "truncated attribute stream should produce empty index"
        );
    }

    #[test]
    fn build_index_truncated_attr_len2_pos1_skips_safely() {
        // Specifically kills the `+ → *` mutant at line 304.
        // pos=1, len=2 → pos+len=3 > data.len()=2 (breaks).
        // `+ → *` mutant: pos*len=1*2=2 ≤ data.len()=2 → does NOT break → reads data[1..3] → panic.
        //
        // data = [header_byte, one_extra_byte]
        //   header_byte = 0x09 = (ATTR_MODULE<<3)|(2-1) = 0x08|0x01 → kind=1, len=2
        // locs_offset=0, locs_size=2, str_offset=2, data.len()=2
        let data = vec![0x09u8, 0x42];
        let index = build_index(&data, 0, 2, 2);
        assert!(
            index.is_empty(),
            "truncated len=2 at pos=1 must not panic and should be empty"
        );
    }

    // -----------------------------------------------------------------------
    // JImageReader::open — data.len() check (line 117: < → ==, < → <=)
    // -----------------------------------------------------------------------

    #[test]
    fn open_minimal_jimage_exact_data_offset_ok() {
        // Build a minimal jimage: table_length=0, locations_size=0, strings_size=0.
        // data_offset = HEADER_SIZE + 0 + 0 + 0 = 28, so a 28-byte file is the minimum.
        // `< → ==` mutant rejects data.len() == data_offset (28 == 28).
        // `< → <=` mutant rejects data.len() <= data_offset.
        let mut buf = vec![0u8; 28];
        buf[0..4].copy_from_slice(&JIMAGE_MAGIC.to_le_bytes());
        buf[4..8].copy_from_slice(&JIMAGE_VERSION.to_le_bytes());
        // flags=0, resource_count=0, table_length=0, locations_size=0, strings_size=0 (all zero)

        let tmp = std::env::temp_dir().join("duke_test_minimal.jimage");
        std::fs::write(&tmp, &buf).expect("write temp jimage");
        let result = JImageReader::open(&tmp);
        let _ = std::fs::remove_file(&tmp);

        assert!(result.is_ok(), "minimal 28-byte jimage should open");
    }

    // -----------------------------------------------------------------------
    // resource_count (line 140: replace return with 1)
    // -----------------------------------------------------------------------

    #[test]
    fn resource_count_reflects_header_value() {
        // Build a jimage with resource_count=7; verify resource_count() returns 7, not 1.
        let mut buf = vec![0u8; 28];
        buf[0..4].copy_from_slice(&JIMAGE_MAGIC.to_le_bytes());
        buf[4..8].copy_from_slice(&JIMAGE_VERSION.to_le_bytes());
        buf[12..16].copy_from_slice(&7u32.to_le_bytes()); // resource_count = 7

        let tmp = std::env::temp_dir().join("duke_test_rc7.jimage");
        std::fs::write(&tmp, &buf).expect("write temp jimage");
        let reader = JImageReader::open(&tmp).expect("open");
        let _ = std::fs::remove_file(&tmp);

        assert_eq!(
            reader.resource_count(),
            7,
            "resource_count() must return header value"
        );
    }

    // -----------------------------------------------------------------------
    // build_jimage_path (line 358: || → &&)
    // -----------------------------------------------------------------------

    #[test]
    fn build_jimage_path_empty_module_returns_empty() {
        // module="" → return "". Mutant `&&` only returns "" if BOTH empty.
        assert!(build_jimage_path("", "parent", "base", "ext").is_empty());
    }

    #[test]
    fn build_jimage_path_empty_base_returns_empty() {
        // base="" → return "". Mutant `&&` only returns "" if BOTH empty.
        assert!(build_jimage_path("module", "parent", "", "ext").is_empty());
    }

    // -----------------------------------------------------------------------
    // read_resource (lines 164, 174, 182)
    // -----------------------------------------------------------------------

    fn make_reader(data: Vec<u8>, compressed: u64, uncompressed: u64) -> JImageReader {
        let mut index = HashMap::new();
        index.insert(
            "r".to_string(),
            ResourceInfo {
                offset: 0,
                compressed,
                uncompressed,
            },
        );
        JImageReader {
            data,
            resource_count: 1,
            data_offset: 0,
            index,
        }
    }

    #[test]
    fn read_resource_uses_compressed_len_for_bounds_check() {
        // compressed=3, uncompressed=1000. data.len()=6 is enough for compressed but not uncompressed.
        // Original: raw_len=3 → bounds pass → decompresses [0xFF,0xFF,0xFF] → Decompress error.
        // Mutant (compressed > 0 → < 0, always false): raw_len=1000 → bounds fail → JImageFormat.
        let reader = make_reader(vec![0xFF, 0xFF, 0xFF, 0, 0, 0], 3, 1000);
        let result = reader.read_resource("r");
        assert!(
            !matches!(result, Err(LoadError::JImageFormat { .. })),
            "should use compressed length (3), not uncompressed (1000)"
        );
    }

    #[test]
    fn read_resource_decompresses_when_compressed_nonzero() {
        // compressed=2, data=[0xFF, 0xFF] — BTYPE=11 reserved → invalid deflate → Decompress error.
        // Original: decompresses → Err(Decompress).
        // Mutant (compressed > 0 → < 0): skips decompression → Ok([0xFF, 0xFF]).
        let reader = make_reader(vec![0xFF, 0xFF, 0, 0, 0], 2, 5);
        let result = reader.read_resource("r");
        assert!(
            matches!(result, Err(LoadError::Decompress { .. })),
            "compressed > 0 must trigger decompression: {result:?}"
        );
    }

    #[test]
    fn read_resource_exact_boundary_succeeds() {
        // uncompressed=5, data.len()=5 → end==data.len(), original passes.
        // Mutant `> → >=`: end >= data.len() → JImageFormat error.
        let reader = make_reader(vec![1, 2, 3, 4, 5], 0, 5);
        let result = reader.read_resource("r");
        assert!(
            matches!(result, Ok(ref v) if v == &[1, 2, 3, 4, 5]),
            "resource ending exactly at data boundary must succeed"
        );
    }

    #[test]
    fn read_resource_rejects_capacity_overflow() {
        // Attack: Provide an uncompressed size of u64::MAX.
        // If unpatched, Vec::with_capacity(usize::MAX) will panic with a capacity overflow.
        // Uncompressed is max, compressed is 1 byte, so bounds check passes.
        let reader = make_reader(vec![0xFF, 0xFF, 0, 0, 0], 2, u64::MAX);
        let result = reader.read_resource("r");
        assert!(
            matches!(result, Err(LoadError::Decompress { .. })),
            "should safely fail decompression, not panic with capacity overflow"
        );
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashMap;

    proptest! {
        #[test]
        fn fuzz_jimage_resource_info(
            offset in any::<u64>(),
            compressed in any::<u64>(),
            uncompressed in any::<u64>()
        ) {
            let mut index = HashMap::new();
            index.insert("test".to_string(), ResourceInfo {
                offset,
                compressed,
                uncompressed,
            });

            let reader = JImageReader {
                data: vec![0; 100],
                resource_count: 1,
                data_offset: 0,
                index,
            };

            let _ = reader.read_resource("test");
        }
    }
}
