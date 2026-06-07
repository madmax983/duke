#![allow(missing_docs)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::uninlined_format_args)]

use duke_loader::ZipReader;

#[test]
fn test_zip_bomb_bypass() {
    let mut zip = Vec::new();

    // We create a "bomb" where compressed size is small, but decompresses to 20MB.
    // However, we trick the limit check by setting uncompressed_size = 0.
    // Limit is 256MB. It will bypass the initial capacity check.
    // The decoder will run and we'll see if it crashes or fully processes up to 20MB.

    let uncompressed = vec![b'A'; 1024 * 1024 * 20]; // 20 MB

    use flate2::Compression;
    use flate2::write::DeflateEncoder;
    use std::io::Write;

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&uncompressed).unwrap();
    let compressed = encoder.finish().unwrap();

    let name = "bomb.txt";
    let name_bytes = name.as_bytes();

    zip.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]);
    zip.extend_from_slice(&[20, 0]); // version
    zip.extend_from_slice(&[0, 0]); // flags
    zip.extend_from_slice(&[8, 0]); // compression = DEFLATE
    zip.extend_from_slice(&[0, 0, 0, 0]); // time
    zip.extend_from_slice(&[0, 0, 0, 0]); // crc32

    let compressed_size = (compressed.len() as u32).to_le_bytes();
    zip.extend_from_slice(&compressed_size); // compressed size

    let uncompressed_size = (0u32).to_le_bytes(); // Trick: set to 0
    zip.extend_from_slice(&uncompressed_size); // uncompressed size

    let name_len = (name_bytes.len() as u16).to_le_bytes();
    zip.extend_from_slice(&name_len); // name len

    zip.extend_from_slice(&[0, 0]); // extra len

    zip.extend_from_slice(name_bytes);
    zip.extend_from_slice(&compressed);

    let offset = zip.len();

    // CD header
    zip.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
    zip.extend_from_slice(&[20, 0]); // version made by
    zip.extend_from_slice(&[20, 0]); // version needed
    zip.extend_from_slice(&[0, 0]); // flags
    zip.extend_from_slice(&[8, 0]); // compression = DEFLATE
    zip.extend_from_slice(&[0, 0, 0, 0]); // time
    zip.extend_from_slice(&[0, 0, 0, 0]); // crc32
    zip.extend_from_slice(&compressed_size); // compressed size
    zip.extend_from_slice(&uncompressed_size); // uncompressed size
    zip.extend_from_slice(&name_len); // name len
    zip.extend_from_slice(&[0, 0]); // extra len
    zip.extend_from_slice(&[0, 0]); // comment len
    zip.extend_from_slice(&[0, 0]); // disk
    zip.extend_from_slice(&[0, 0]); // attr
    zip.extend_from_slice(&[0, 0, 0, 0]); // ext attr
    zip.extend_from_slice(&[0, 0, 0, 0]); // offset
    zip.extend_from_slice(name_bytes);

    let cd_size = zip.len() - offset;

    // EOCD
    zip.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
    zip.extend_from_slice(&[0, 0, 0, 0]);
    zip.extend_from_slice(&[1, 0, 1, 0]); // 1 entry
    zip.extend_from_slice(&(cd_size as u32).to_le_bytes()); // cd size
    zip.extend_from_slice(&(offset as u32).to_le_bytes()); // cd offset
    zip.extend_from_slice(&[0, 0]); // comment len

    let reader = ZipReader::from_bytes(zip).unwrap();
    let err = reader.read_entry("bomb.txt").unwrap_err();
    println!("Err: {:?}", err);

    // We expect it to fail the CRC check because we wrote 0 for CRC32.
    assert!(matches!(err, duke_loader::Error::ZipCrc32 { .. }));
}
