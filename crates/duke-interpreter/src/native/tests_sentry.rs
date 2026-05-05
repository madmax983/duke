#[cfg(test)]
mod charset_codec_tests {
    use super::*;
    #[test]
    fn charset_aliases_map_to_canonical_variants() {
        assert_eq!(charset_for_name("utf8"), Some(StandardCharset::Utf8));
        assert_eq!(charset_for_name("latin1"), Some(StandardCharset::Iso88591));
        assert_eq!(charset_for_name("ASCII"), Some(StandardCharset::UsAscii));
        assert_eq!(charset_for_name("not-a-charset"), None);
    }
    #[test]
    fn utf8_decode_replaces_malformed_sequence() {
        assert_eq!(
            decode_string_with_charset(& [0xc3, 0x28], StandardCharset::Utf8),
            "\u{fffd}("
        );
    }
    #[test]
    fn utf16_encodes_bom_and_decodes_surrogate_pair() {
        let value = decode_string_with_charset(
            &[0xf0, 0x9f, 0x98, 0x80, b' ', b'e', b'm', b'o', b'j', b'i'],
            StandardCharset::Utf8,
        );
        let bytes = encode_string_with_charset(&value, StandardCharset::Utf16);
        assert_eq!(& bytes[0..2], & [0xfe, 0xff]);
        assert_eq!(decode_string_with_charset(& bytes, StandardCharset::Utf16), value);
    }
    #[test]
    fn ascii_and_latin1_encode_unmappable_as_question_mark() {
        assert_eq!(
            encode_string_with_charset("\u{20ac}", StandardCharset::UsAscii), vec![b'?']
        );
        assert_eq!(
            encode_string_with_charset("\u{20ac}", StandardCharset::Iso88591), vec![b'?']
        );
    }
    #[test]
    fn standard_charset_allocation_is_canonical_per_heap() {
        let mut heap = duke_gc::Heap::new();
        let first = allocate_standard_charset(&mut heap, "UTF-8");
        let second = allocate_standard_charset(&mut heap, "UTF-8");
        assert_eq!(first, second);
        assert_eq!(
            heap.get(first).expect("charset object").string_value.as_deref(),
            Some("UTF-8")
        );
    }
}
#[cfg(test)]
mod havoc_proptest_math {
    use super::*;
    use proptest::prelude::*;
    use std::io::sink;
    proptest! {
        #[test] fn fuzz_native_math_floor_div_int(a in any::< i32 > (), b in any::< i32 >
        ()) { let mut heap = duke_gc::Heap::new(); let mut control =
        NativeControl::default(); let args = vec![Slot::Int(a), Slot::Int(b)]; let result
        = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { let _ =
        native_math_floor_div_int(& args, & mut heap, & mut sink(), & mut control); }));
        assert!(result.is_ok(), "Panic on a={a}, b={b}"); }
    }
}
#[cfg(test)]
mod havoc_thread_join_itself {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;
    #[test]
    fn test_join_java_thread_itself() {
        let runtime = Arc::new(Mutex::new(CompletionRuntime::default()));
        let runtime_clone = runtime.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let (tx_panic, rx_panic) = std::sync::mpsc::channel();
        let handle = thread::spawn(move || -> Result<()> {
            rx.recv().unwrap();
            let result = std::panic::catch_unwind(
                std::panic::AssertUnwindSafe(|| {
                    let _ = join_java_thread(&runtime_clone, 0);
                }),
            );
            if let Err(e) = result {
                if let Some(s) = e.downcast_ref::<&str>() {
                    tx_panic.send((*s).to_string()).unwrap();
                } else if let Some(s) = e.downcast_ref::<String>() {
                    tx_panic.send(s.clone()).unwrap();
                }
            }
            Ok(())
        });
        {
            let mut rt = runtime
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            rt.handles.insert(0, handle);
            let mut record = crate::threading::ThreadRecord::new(123, 0);
            record.finished = false;
            rt.threads.register(record);
        }
        tx.send(()).unwrap();
        let res = rx_panic.recv_timeout(std::time::Duration::from_millis(50));
        assert!(res.is_err(), "Expected no panic, but received one!");
    }
}
#[cfg(test)]
mod havoc_string_indent_overflow {
    use super::*;
    use std::io::sink;
    use duke_gc::Heap;
    use duke_runtime::Slot;
    #[test]
    fn test_string_indent_overflow() {
        let mut heap = Heap::new();
        let r = heap.allocate_string("hello\nworld".to_string());
        let args = vec![Slot::Reference(Some(r)), Slot::Int(i32::MIN)];
        let mut control = NativeControl::default();
        let _ = native_string_indent(&args, &mut heap, &mut sink(), &mut control);
    }
}
#[cfg(test)]
mod tests_zip_coverage {
    use super::*;
    #[test]
    fn zip_registry_error_coverage() {
        let err = zip_entry_count(999).unwrap_err();
        assert!(matches!(err, Error::JavaException { .. }));
        let err = zip_get_entry_info(999, "test").unwrap_err();
        assert!(matches!(err, Error::JavaException { .. }));
        let err = zip_read_entry(999, "test").unwrap_err();
        assert!(matches!(err, Error::JavaException { .. }));
        zip_close(999);
    }
}
#[cfg(test)]
mod tests_zip_open_coverage {
    use super::*;
    #[test]
    fn zip_open_io_error() {
        let err = zip_open(std::path::Path::new("/does/not/exist/ever/zip.zip"))
            .unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name ==
            "java/io/FileNotFoundException")
        );
    }
    #[test]
    fn zip_open_format_error() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("bad_zip_format.zip");
        std::fs::write(&path, b"not a zip file").unwrap();
        let err = zip_open(&path).unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name ==
            "java/util/zip/ZipException")
        );
        std::fs::remove_file(&path).unwrap();
    }
    #[cfg(test)]
    mod havoc_coverage_tests {
        use super::*;
        #[test]
        fn test_system_property_value_fallback() {
            assert!(system_property_value_fallback("file.separator").is_some());
            assert!(system_property_value_fallback("path.separator").is_some());
            assert!(system_property_value_fallback("line.separator").is_some());
            assert!(system_property_value_fallback("os.name").is_some());
            assert!(system_property_value_fallback("unknown.property").is_none());
            assert!(system_property_value_fallback("java.version").is_some());
            assert!(system_property_value_fallback("user.dir").is_some());
        }
    }
}
#[cfg(test)]
mod havoc_string_repeat_oom {
    use super::*;
    use std::io::sink;
    use duke_gc::Heap;
    use duke_runtime::Slot;
    #[test]
    fn test_string_repeat_oom_trigger() {
        let mut heap = Heap::new();
        let r = heap.allocate_string("12345678901234567890".to_string());
        let args = vec![Slot::Reference(Some(r)), Slot::Int(i32::MAX)];
        let mut control = NativeControl::default();
        let result = native_string_repeat(&args, &mut heap, &mut sink(), &mut control);
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name ==
            "java/lang/OutOfMemoryError")
        );
    }
}
#[cfg(test)]
mod sentry_tests {
    use super::*;
    #[test]
    fn test_zip_functions_error_cases() {
        let invalid_id = -999;
        let count_err = zip_entry_count(invalid_id).unwrap_err();
        assert!(
            matches!(count_err, Error::JavaException { ref class_name } if class_name ==
            "java/io/IOException")
        );
        let info_err = zip_get_entry_info(invalid_id, "test").unwrap_err();
        assert!(
            matches!(info_err, Error::JavaException { ref class_name } if class_name ==
            "java/io/IOException")
        );
        let read_err = zip_read_entry(invalid_id, "test").unwrap_err();
        assert!(
            matches!(read_err, Error::JavaException { ref class_name } if class_name ==
            "java/io/IOException")
        );
        zip_close(invalid_id);
    }
}
#[cfg(test)]
mod havoc_string_indent_overflow_positive {
    use super::*;
    use std::io::sink;
    use duke_gc::Heap;
    use duke_runtime::Slot;
    #[test]
    fn test_string_indent_overflow_trigger() {
        let mut heap = Heap::new();
        let r = heap.allocate_string("hello\nworld".to_string());
        let args = vec![Slot::Reference(Some(r)), Slot::Int(i32::MAX)];
        let mut control = NativeControl::default();
        let result = native_string_indent(&args, &mut heap, &mut sink(), &mut control);
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::JavaException { ref class_name } if class_name ==
            "java/lang/OutOfMemoryError")
        );
    }
}
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
