#[cfg(test)]
mod process_tests {
    use super::*;

    #[test]
    fn test_process_wait_invalid() {
        let mut gc = crate::Heap::new();
        // Wait on invalid process ID
        assert!(matches!(
            gc.wait_host_process(999),
            Err(VmError::JavaException { ref class_name }) if class_name == "java/io/IOException"
        ));
    }
}
