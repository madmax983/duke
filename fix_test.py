import re

with open('crates/duke-interpreter/src/native.rs', 'r') as f:
    content = f.read()

# Add a test module to bump coverage on these new functions
test_code = """
#[cfg(test)]
mod tests_zip_coverage {
    use super::*;

    #[test]
    fn zip_registry_error_coverage() {
        // ID 999 doesn't exist
        let err = zip_entry_count(999).unwrap_err();
        assert!(matches!(err, VmError::JavaException { .. }));

        let err = zip_get_entry_info(999, "test").unwrap_err();
        assert!(matches!(err, VmError::JavaException { .. }));

        let err = zip_read_entry(999, "test").unwrap_err();
        assert!(matches!(err, VmError::JavaException { .. }));

        // Removing non-existent shouldn't panic
        zip_close(999);
    }
}
"""
content += test_code

with open('crates/duke-interpreter/src/native.rs', 'w') as f:
    f.write(content)
