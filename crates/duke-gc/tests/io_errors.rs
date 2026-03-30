use duke_gc::Heap;



#[test]
fn test_host_file_operations_invalid_handles() {
    let mut heap = Heap::new();

    // Test invalid read
    let res = heap.read_host_file_byte(999);
    assert!(res.is_err());

    // Test invalid write
    let res = heap.write_host_file_byte(999, 0);
    assert!(res.is_err());

    // Test invalid close (should silently ignore)
    heap.close_host_file(999);
}

#[test]
fn test_io_errors_via_permissions() {
    let mut heap = Heap::new();

    // Test write_host_file_byte IO error
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("read_only_file.txt");

    std::fs::File::create(&file_path).unwrap();
    let mut perms = std::fs::metadata(&file_path).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&file_path, perms).unwrap();

    let out_res = heap.open_host_output_file(&file_path);
    assert!(out_res.is_err());
}

#[test]
fn test_read_host_file_byte_io_error_mock() {
    let mut heap = Heap::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("dir");
    std::fs::create_dir(&file_path).unwrap();

    // Some OS might allow opening dir for read, some might error.
    let in_res = heap.open_host_input_file(&file_path);
    if let Ok(in_id) = in_res {
        // trying to read from a directory usually fails with Is a directory
        let res = heap.read_host_file_byte(in_id);
        assert!(res.is_err());
    }
}

#[test]
fn test_open_host_input_file_not_found() {
    let mut heap = Heap::new();
    let res = heap.open_host_input_file(std::path::Path::new(
        "/non/existent/file/path/that/should/not/exist",
    ));
    assert!(res.is_err());
    match res.unwrap_err() {
        duke_runtime::error::VmError::JavaException { class_name } => {
            assert_eq!(class_name, "java/io/FileNotFoundException");
        }
        _ => panic!("Expected JavaException"),
    }
}

#[test]
fn test_write_host_file_wrong_handle() {
    let mut heap = Heap::new();
    let temp_in = tempfile::NamedTempFile::new().unwrap();
    let in_id = heap.open_host_input_file(temp_in.path()).unwrap();

    // This will hit the `let HostFileHandle::Writer(file) = handle else { return Err(...) }` branch
    let res = heap.write_host_file_byte(in_id, b'a' as i32);
    assert!(res.is_err());
    match res.unwrap_err() {
        duke_runtime::error::VmError::JavaException { class_name } => {
            assert_eq!(class_name, "java/io/IOException");
        }
        _ => panic!("Expected JavaException"),
    }
}

#[test]
fn test_read_host_file_wrong_handle() {
    let mut heap = Heap::new();
    let temp_out = tempfile::NamedTempFile::new().unwrap();
    let out_id = heap.open_host_output_file(temp_out.path()).unwrap();

    // This will hit the `let HostFileHandle::Reader(file) = handle else { return Err(...) }` branch
    let res = heap.read_host_file_byte(out_id);
    assert!(res.is_err());
    match res.unwrap_err() {
        duke_runtime::error::VmError::JavaException { class_name } => {
            assert_eq!(class_name, "java/io/IOException");
        }
        _ => panic!("Expected JavaException"),
    }
}
