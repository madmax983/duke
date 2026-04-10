#[cfg(test)]
mod host_file_tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_host_file_io_errors() {
        let mut gc = crate::Heap::new();

        let res = gc.open_host_input_file(std::path::Path::new("/non/existent/file.txt"));
        assert!(matches!(res, Err(VmError::JavaException { ref class_name }) if class_name == "java/io/FileNotFoundException"));

        let res = gc.write_host_file_byte(999, b'A' as i32);
        assert!(matches!(res, Err(VmError::JavaException { ref class_name }) if class_name == "java/io/IOException"));

        let res = gc.read_host_file_byte(999);
        assert!(matches!(res, Err(VmError::JavaException { ref class_name }) if class_name == "java/io/IOException"));

        let file = NamedTempFile::new().unwrap();
        let reader_id = gc.open_host_input_file(file.path()).unwrap();
        let res = gc.write_host_file_byte(reader_id, b'B' as i32);
        assert!(matches!(res, Err(VmError::JavaException { ref class_name }) if class_name == "java/io/IOException"));

        let writer_id = gc.open_host_output_file(file.path()).unwrap();
        let res = gc.read_host_file_byte(writer_id);
        assert!(matches!(res, Err(VmError::JavaException { ref class_name }) if class_name == "java/io/IOException"));

        let res = gc.open_host_output_file(std::path::Path::new("/non/existent/dir/file.txt"));
        assert!(matches!(res, Err(VmError::JavaException { ref class_name }) if class_name == "java/io/IOException"));
    }
}
