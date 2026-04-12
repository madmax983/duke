//! Read-only ZIP/JAR archive support.
//!
//! Implements index-on-open, lazy decompression — the same pattern as
//! [`super::jimage::JImageReader`].  Used both by the internal classloader
//! (`ZipLoader`) and by the Java-space `ZipFile` native bridges.

pub mod loader;
pub mod reader;

pub use loader::ZipLoader;
pub use reader::{ZipEntryInfo, ZipReader};

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::cast_possible_truncation)]
mod tests {
    use super::reader::*;
    use super::*;
    use crate::zip::reader::{LOCAL_SIGNATURE, METHOD_DEFLATED};
    use crate::{ClassLoader, LoadError};
    use std::path::Path;

    #[test]
    fn test_zip_loader_reader() {
        let zip_bytes = build_stored_zip("test.txt", b"hello world");
        let reader = ZipReader::from_bytes(zip_bytes).expect("valid zip");
        let loader = ZipLoader::from_reader(reader).expect("valid zip");
        let r = loader.reader();
        assert_eq!(r.data().len(), 125);
    }

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
        corrupted_info.local_header_offset = (reader.data().len() - 10) as u64;

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
        corrupted_info.compressed_size = (reader.data().len() + 10) as u64;

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
            if let LoadError::ZipFormat { msg } = err {
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

    #[test]
    fn test_zip_missing_eocd() {
        let zip_bytes = vec![0u8; 100];
        let err = ZipReader::from_bytes(zip_bytes).unwrap_err();
        assert!(
            matches!(err, LoadError::ZipFormat { ref msg } if msg == "could not find end-of-central-directory record")
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
            matches!(err, LoadError::ZipFormat { ref msg } if msg == "central directory extends past end of file")
        );
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::zip::reader::{LOCAL_SIGNATURE, METHOD_DEFLATED};
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
