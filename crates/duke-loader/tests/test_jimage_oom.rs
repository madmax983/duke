use duke_loader::jimage::JImageReader;
use std::fs::File;
use std::io::Write;

#[test]
fn test_jimage_oom() {
    let test_file_path = std::env::temp_dir().join("test_oom.jimage");
    let mut file = File::create(&test_file_path).unwrap();
    let tl: usize = 1;
    let ls: usize = 128;
    let ss: usize = 128;

    let mut data = vec![
        0xDA, 0xDA, 0xFE, 0xCA, // JIMAGE_MAGIC
        0x00, 0x00, 0x01, 0x00, // JIMAGE_VERSION
        0x00, 0x00, 0x00, 0x00, // flags
        0x01, 0x00, 0x00, 0x00, // resource_count
        tl as u8, 0x00, 0x00, 0x00, // table_length
        ls as u8, 0x00, 0x00, 0x00, // locations_size
        ss as u8, 0x00, 0x00, 0x00, // strings_size
    ];

    // header is 28 bytes
    // tables offset is 28
    // redirect tl * 4
    for _ in 0..tl {
        data.extend_from_slice(&[0, 0, 0, 0]);
    }
    // offsets tl * 4
    for _ in 0..tl {
        data.extend_from_slice(&[0, 0, 0, 0]);
    }

    let base_idx = data.len();

    // ATTR_MODULE
    data.push(1 << 3);
    data.push(1); // string index 1 ("a")

    // ATTR_PARENT
    data.push(2 << 3);
    data.push(1);

    // base = string offset 1
    data.push(3 << 3);
    data.push(1);

    // ext
    data.push(4 << 3);
    data.push(0);

    // compressed size (1 byte long) = 1
    data.push(6 << 3);
    data.push(1);

    // uncompressed size (4 bytes long) = 0xFFFFFFFF (4 GB)
    data.push((7 << 3) | 7);
    data.extend_from_slice(&[0x03, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]); // 2^58

    // offset (1 byte long) = 0
    data.push(5 << 3);
    data.push(0);

    // end
    data.push(0);

    // pad locations to ls
    while data.len() < base_idx + ls {
        data.push(0);
    }

    let str_idx = data.len();

    // strings table ss (128 bytes)
    // index 0 is empty string
    data.push(0);
    // index 1 is "a"
    data.extend_from_slice(b"a\0");

    // pad strings
    while data.len() < str_idx + ss {
        data.push(0);
    }

    // data section (1 byte compressed data)
    data.push(0x42);

    file.write_all(&data).unwrap();
    drop(file);

    let reader = JImageReader::open(&test_file_path).unwrap();

    let res = reader.find_resource("/a/a/a");
    assert!(res.is_some());
    let _ = reader.read_resource("/a/a/a");

    let _ = std::fs::remove_file(test_file_path);
}
