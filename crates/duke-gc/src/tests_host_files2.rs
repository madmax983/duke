#[cfg(test)]
mod host_file_tests2 {
    use super::*;

    #[test]
    fn test_open_input_file_other_io_error() {
        let mut gc = crate::Heap::new();
        // Opening a directory should trigger an IO error that is mapped correctly,
        // though typically it might be IsADirectory or similar depending on platform.
        // Let's just create a directory and try to open it as a file.
        let temp_dir = tempfile::tempdir().unwrap();
        let res = gc.open_host_input_file(temp_dir.path());
        assert!(matches!(res, Err(VmError::JavaException { ref class_name }) if class_name == "java/io/IOException" || class_name == "java/io/FileNotFoundException"));
    }

    #[test]
    fn test_open_output_file_io_error() {
        let mut gc = crate::Heap::new();
        // Try creating a file in a non-existent directory
        let res = gc.open_host_output_file(std::path::Path::new("/non/existent/dir/file.txt"));
        assert!(matches!(res, Err(VmError::JavaException { ref class_name }) if class_name == "java/io/IOException"));
    }
}
