
#[cfg(test)]
mod native_helper_tests {
    use super::*;

    #[test]
    fn should_return_error_when_extract_ref_arg_receives_int() {
        let args = vec![Slot::Int(42)];
        let res = extract_ref_arg(&args, 0);
        assert!(matches!(res, Err(Error::NullPointerException)));
    }

    #[test]
    fn should_return_error_when_extract_ref_arg_out_of_bounds() {
        let args = vec![];
        let res = extract_ref_arg(&args, 0);
        assert!(matches!(res, Err(Error::NullPointerException)));
    }

    #[test]
    fn should_return_error_when_extract_io_fd_receives_no_fields() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/lang/Object".to_string(), 0);
        let res = extract_io_fd(&heap, obj_ref);
        assert!(matches!(res, Err(Error::JavaException { .. })));
    }

    #[test]
    fn should_return_error_when_extract_io_fd_at_receives_no_fields() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/lang/Object".to_string(), 0);
        let res = extract_io_fd_at(&heap, obj_ref, 10);
        assert!(matches!(res, Err(Error::JavaException { .. })));
    }

    #[test]
    fn should_return_error_when_extract_int_arg_receives_ref() {
        let args = vec![Slot::Reference(None)];
        let res = extract_int_arg(&args, 0);
        assert!(matches!(res, Err(Error::TypeMismatch { .. })));
    }

    #[test]
    fn should_return_error_when_extract_int_arg_out_of_bounds() {
        let args = vec![];
        let res = extract_int_arg(&args, 0);
        assert!(matches!(res, Err(Error::TypeMismatch { .. })));
    }

    #[test]
    fn should_extract_null_for_out_of_bounds_field_arg() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/lang/Object".to_string(), 0);
        let res = extract_field_arg(&heap, obj_ref, 5).unwrap();
        assert_eq!(res, Slot::Reference(None));
    }

    #[test]
    fn should_extract_null_for_empty_first_field_arg() {
        let mut heap = duke_gc::Heap::new();
        let obj_ref = heap.allocate("java/lang/Object".to_string(), 0);
        let res = extract_first_field_arg(&heap, obj_ref).unwrap();
        assert_eq!(res, Slot::Reference(None));
    }

    #[test]
    fn should_extract_null_for_out_of_bounds_slot_arg() {
        let args = vec![];
        let res = extract_slot_arg(&args, 0);
        assert_eq!(res, Slot::Reference(None));
    }
}
