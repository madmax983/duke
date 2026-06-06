#![allow(missing_docs)]

use duke_loader::ZipLoader;
use std::io::Write;
use std::path::PathBuf;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::FileOptions;

#[test]
fn havoc_zip_bomb_recursion_stack_overflow() {
    let p = PathBuf::from("temp_bomb_fuzz.zip");

    // Create nested zips
    let mut current_buf = Vec::new();
    {
        let mut zw = ZipWriter::new(std::io::Cursor::new(&mut current_buf));
        let options: FileOptions<()> =
            FileOptions::default().compression_method(CompressionMethod::Stored);
        zw.start_file("dummy.txt", options).unwrap();
        zw.write_all(b"dummy").unwrap();
        zw.finish().unwrap();
    }

    // We create ~500 layers of nesting, which causes a stack overflow on ZipLoader::open
    for _ in 0..500 {
        let mut next_buf = Vec::new();
        {
            let mut zw = ZipWriter::new(std::io::Cursor::new(&mut next_buf));
            let options: FileOptions<()> =
                FileOptions::default().compression_method(CompressionMethod::Stored);
            zw.start_file("BOOT-INF/lib/a.jar", options).unwrap();
            zw.write_all(&current_buf).unwrap();
            zw.finish().unwrap();
        }
        current_buf = next_buf;
    }

    std::fs::write(&p, current_buf).unwrap();
    let res = ZipLoader::open(&p);

    let _ = std::fs::remove_file(&p);

    assert!(
        res.is_err(),
        "Vulnerability triggered! System did not reject the ZIP bomb"
    );
}
