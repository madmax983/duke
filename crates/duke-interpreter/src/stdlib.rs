use duke_runtime::Slot;
use crate::{ClassContext, ClassRegistry, FieldEntry};
use crate::{native_println_string, native_println_int, native_println_void, native_file_init, native_file_exists, native_file_is_file, native_file_is_directory, native_process_get_input_stream, native_process_get_error_stream, native_process_get_output_stream, native_process_wait_for, native_process_exit_value, native_process_destroy, native_process_builder_init, native_process_builder_directory, native_process_builder_start, native_runtime_get_runtime, native_runtime_exec_array, native_runtime_exec_array_dir, native_file_input_stream_read, native_file_input_stream_read_bytes, native_file_input_stream_close, native_file_output_stream_write, native_file_output_stream_write_bytes, native_file_output_stream_close, native_file_input_stream_init, native_file_output_stream_init, native_server_socket_init, native_server_socket_accept, native_server_socket_get_local_port, native_server_socket_close, native_socket_init, native_socket_get_input_stream, native_socket_get_output_stream, native_socket_close, native_string_length, native_string_equals, native_string_char_at, native_string_substring, native_string_substring_range, native_string_indexof, native_string_contains, native_string_isempty, native_string_compareto, native_string_compareto_object, native_string_startswith, native_string_endswith, native_string_trim, native_string_tochararray, native_string_touppercase, native_string_tolowercase, native_string_replace_char, native_string_replace_charsequence, native_string_split, native_string_hashcode, native_string_tostring, native_string_value_of_int, native_print_string, native_print_int, native_println_long, native_println_float, native_println_double, native_println_boolean, native_println_char, native_println_object, native_print_long, native_print_float, native_print_double, native_print_boolean, native_print_char, native_print_object, native_system_exit, native_system_current_time_millis, native_system_nano_time, native_system_get_property, native_system_get_property_with_default, native_system_set_property, native_object_init, native_object_equals, native_object_hashcode, native_object_tostring, native_object_clone, native_object_get_class, native_class_for_name, native_class_for_name_with_loader, native_class_get_name, native_class_get_package_name, native_class_desired_assertion_status, native_class_get_declared_methods, native_class_get_declared_fields, native_class_get_declared_constructors, native_class_get_methods, native_class_get_fields, native_class_get_constructors, native_class_get_declared_method, native_class_get_declared_field, native_class_get_declared_constructor, native_class_get_method, native_class_get_field, native_class_get_constructor, native_class_new_instance, native_class_get_protection_domain, native_class_get_class_loader, native_false_boolean, native_zero_long, native_void_noop, native_protection_domain_get_code_source, native_code_source_get_location, native_url_to_uri, native_url_set_url_stream_handler_factory, native_path_of, native_path_to_file, native_paths_get, native_posix_file_permissions_as_file_attribute, native_boot_archive_entry_name, native_boot_archive_entry_is_directory, native_boot_jar_file_archive_get_class_path_urls, native_boot_archive_get_manifest, native_boot_jar_file_archive_get_manifest, native_boot_exploded_archive_get_class_path_urls, native_boot_exploded_archive_get_manifest, native_boot_launched_class_loader_init, native_reflect_method_get_name, native_reflection_member_get_declaring_class, native_reflect_method_get_return_type, native_reflect_method_invoke, native_reflection_member_set_accessible, native_reflect_method_get_parameter_count, native_reflect_executable_get_parameter_types, native_reflect_constructor_get_name, native_reflect_constructor_new_instance, native_reflect_field_get_name, native_reflect_field_get_type, native_reflect_field_get, native_reflect_field_set, native_class_loader_register_as_parallel_capable, native_thread_init, native_thread_init_runnable, native_thread_current_thread, native_thread_start, native_thread_join, native_thread_sleep, native_throwable_add_suppressed, native_enum_init, native_enum_ordinal, native_enum_name, native_enum_valueof, native_integer_parseint, native_integer_parseint_radix, native_integer_valueof_string, native_integer_valueof_string_radix, native_integer_decode, native_integer_valueof, native_integer_intvalue, native_integer_tostring_static, native_integer_tohexstring_static, native_integer_tooctalstring_static, native_integer_tobinarystring_static, native_integer_tounsignedlong_static, native_integer_compareunsigned_static, native_integer_compareto, native_long_parselong, native_long_parselong_radix, native_long_valueof_string, native_long_valueof_string_radix, native_long_decode, native_long_valueof, native_long_longvalue, native_long_tostring_static, native_long_tohexstring_static, native_long_tooctalstring_static, native_long_tobinarystring_static, native_long_compareunsigned_static, native_long_compareto, native_double_parsedouble, native_double_valueof, native_double_doublevalue, native_double_isnan, native_double_compareto, native_float_parsefloat, native_float_valueof, native_float_floatvalue, native_float_compareto, native_boolean_parseboolean, native_boolean_valueof, native_boolean_booleanvalue, native_boolean_compareto, native_byte_parsebyte, native_byte_parsebyte_radix, native_byte_valueof_string, native_byte_valueof_string_radix, native_byte_valueof, native_byte_bytevalue, native_byte_compareto, native_short_parseshort, native_short_parseshort_radix, native_short_valueof_string, native_short_valueof_string_radix, native_short_valueof, native_short_shortvalue, native_short_compareto, native_math_max_int, native_math_min_int, native_math_abs_int, native_math_floor_mod_int, native_math_sqrt, native_math_pow, native_math_floor, native_math_ceil, native_math_round_double, native_math_abs_long, native_math_abs_double, native_math_max_long, native_math_min_long, native_math_max_double, native_math_min_double, native_string_value_of_long, native_string_value_of_double, native_string_value_of_float, native_string_value_of_boolean, native_string_value_of_char, native_string_value_of_object, native_string_concat, native_string_format, native_sb_init, native_sb_init_string, native_sb_append_string, native_sb_append_int, native_sb_append_long, native_sb_append_double, native_sb_append_float, native_sb_append_boolean, native_sb_append_char, native_sb_tostring, native_sb_length, native_char_is_digit, native_char_is_letter, native_char_is_whitespace, native_char_is_uppercase, native_char_is_lowercase, native_char_to_uppercase, native_char_to_lowercase, native_char_is_letter_or_digit, native_char_valueof, native_char_charvalue, native_char_compareto, native_collection_to_array, native_collection_to_array_with_seed_array, native_arraylist_init, native_arraylist_add, native_arraylist_get, native_arraylist_size, native_arraylist_iterator, array_list_sort, native_arraylist_iter_init, native_arraylist_iter_hasnext, native_arraylist_iter_next, native_arrays_fill_int, native_arrays_fill_object, native_arrays_copyof_int, native_arrays_copyof_object, native_arrays_sort_int, native_hashmap_init, native_hashmap_put, native_hashmap_get, native_hashmap_contains_key, native_hashmap_size, native_hashmap_remove, native_hashmap_is_empty, native_hashmap_get_or_default, native_set_of, native_hashset_init, native_hashset_init_from_collection, native_hashset_add, native_hashset_contains, native_hashset_remove, native_hashset_size, native_hashset_is_empty, native_hashset_iterator, native_hashset_iter_init, native_hashset_iter_hasnext, native_hashset_iter_next, native_collections_sort, native_zip_entry_get_name, native_zip_entry_get_compressed_size, native_zip_entry_get_size, native_zip_entry_get_method, native_zip_file_init, native_zip_file_get_entry, native_zip_file_get_input_stream, native_zip_file_close, native_zip_file_size, native_jar_file_init_from_file, native_jar_file_init_with_mode_and_version, native_jar_file_get_manifest, native_boot_nested_jar_file_get_manifest, native_manifest_get_main_attributes, native_attributes_get_value}; // For the pub(crate) native helpers left in lib.rs


/// Registry of loaded classes — maps class name to its `ClassContext`.
///
/// Used by `execute_class` for cross-class method dispatch.
///
/// # Examples
///
/// A native handler that can call back into the interpreter to invoke Java methods.
///
/// The `invoke` closure takes `heap` and `output` as *parameters* (not captured),
/// using the "loan" pattern: the handler passes its borrows through each call and
/// gets them back when the call returns. Sequential reborrows — no unsafe required.
/// Bootstrap minimal JDK standard library classes for native method support.
///
/// Creates synthetic `java/lang/System` and `java/io/PrintStream` classes and
/// registers native `println` handlers for `(Ljava/lang/String;)V`, `(I)V`,
/// and `()V`.
#[allow(clippy::too_many_lines)]
pub fn bootstrap_stdlib(registry: &mut ClassRegistry, heap: &mut duke_gc::Heap) {
    // Allocate a PrintStream object on the heap.
    let ps_ref = heap.allocate("java/io/PrintStream".to_string(), 0);

    // Create java/lang/System ClassContext with a single static field `out`.
    let system_ctx = ClassContext {
        class_name: "java/lang/System".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "out".to_string(),
            descriptor: "Ljava/io/PrintStream;".to_string(),
            is_static: true,
        }],
        static_fields: vec![Slot::Reference(Some(ps_ref))],
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(system_ctx);

    // Create java/io/PrintStream ClassContext (empty — all methods are native).
    let ps_ctx = ClassContext {
        class_name: "java/io/PrintStream".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(ps_ctx);

    // Register native println handlers.
    registry.natives_mut().register(
        "java/io/PrintStream",
        "println",
        "(Ljava/lang/String;)V",
        native_println_string,
    );
    registry
        .natives_mut()
        .register("java/io/PrintStream", "println", "(I)V", native_println_int);
    registry
        .natives_mut()
        .register("java/io/PrintStream", "println", "()V", native_println_void);

    let file_ctx = ClassContext {
        class_name: "java/io/File".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "path".to_string(),
            descriptor: "Ljava/lang/String;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(file_ctx);
    registry.natives_mut().register(
        "java/io/File",
        "<init>",
        "(Ljava/lang/String;)V",
        native_file_init,
    );
    registry
        .natives_mut()
        .register("java/io/File", "exists", "()Z", native_file_exists);
    registry
        .natives_mut()
        .register("java/io/File", "isFile", "()Z", native_file_is_file);
    registry.natives_mut().register(
        "java/io/File",
        "isDirectory",
        "()Z",
        native_file_is_directory,
    );

    let io_exception_ctx = ClassContext {
        class_name: "java/io/IOException".to_string(),
        super_class: Some("java/lang/Exception".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(io_exception_ctx);

    let file_not_found_ctx = ClassContext {
        class_name: "java/io/FileNotFoundException".to_string(),
        super_class: Some("java/io/IOException".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(file_not_found_ctx);

    let socket_exception_ctx = ClassContext {
        class_name: "java/net/SocketException".to_string(),
        super_class: Some("java/io/IOException".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(socket_exception_ctx);

    let bind_exception_ctx = ClassContext {
        class_name: "java/net/BindException".to_string(),
        super_class: Some("java/net/SocketException".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(bind_exception_ctx);

    let connect_exception_ctx = ClassContext {
        class_name: "java/net/ConnectException".to_string(),
        super_class: Some("java/net/SocketException".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(connect_exception_ctx);

    let input_stream_ctx = ClassContext {
        class_name: "java/io/InputStream".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/lang/AutoCloseable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(input_stream_ctx);

    let output_stream_ctx = ClassContext {
        class_name: "java/io/OutputStream".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/lang/AutoCloseable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(output_stream_ctx);

    let process_ctx = ClassContext {
        class_name: "java/lang/Process".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(process_ctx);

    let process_impl_ctx = ClassContext {
        class_name: "java/lang/ProcessImpl".to_string(),
        super_class: Some("java/lang/Process".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "pid".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "stdinFd".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "stdoutFd".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "stderrFd".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 4,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(process_impl_ctx);
    registry.natives_mut().register(
        "java/lang/ProcessImpl",
        "getInputStream",
        "()Ljava/io/InputStream;",
        native_process_get_input_stream,
    );
    registry.natives_mut().register(
        "java/lang/ProcessImpl",
        "getErrorStream",
        "()Ljava/io/InputStream;",
        native_process_get_error_stream,
    );
    registry.natives_mut().register(
        "java/lang/ProcessImpl",
        "getOutputStream",
        "()Ljava/io/OutputStream;",
        native_process_get_output_stream,
    );
    registry.natives_mut().register(
        "java/lang/ProcessImpl",
        "waitFor",
        "()I",
        native_process_wait_for,
    );
    registry.natives_mut().register(
        "java/lang/ProcessImpl",
        "exitValue",
        "()I",
        native_process_exit_value,
    );
    registry.natives_mut().register(
        "java/lang/ProcessImpl",
        "destroy",
        "()V",
        native_process_destroy,
    );

    let process_builder_ctx = ClassContext {
        class_name: "java/lang/ProcessBuilder".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "command".to_string(),
                descriptor: "[Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "directory".to_string(),
                descriptor: "Ljava/io/File;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(process_builder_ctx);
    registry.natives_mut().register(
        "java/lang/ProcessBuilder",
        "<init>",
        "([Ljava/lang/String;)V",
        native_process_builder_init,
    );
    registry.natives_mut().register(
        "java/lang/ProcessBuilder",
        "directory",
        "(Ljava/io/File;)Ljava/lang/ProcessBuilder;",
        native_process_builder_directory,
    );
    registry.natives_mut().register(
        "java/lang/ProcessBuilder",
        "start",
        "()Ljava/lang/Process;",
        native_process_builder_start,
    );

    let runtime_ctx = ClassContext {
        class_name: "java/lang/Runtime".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(runtime_ctx);
    registry.natives_mut().register(
        "java/lang/Runtime",
        "getRuntime",
        "()Ljava/lang/Runtime;",
        native_runtime_get_runtime,
    );
    registry.natives_mut().register(
        "java/lang/Runtime",
        "exec",
        "([Ljava/lang/String;)Ljava/lang/Process;",
        native_runtime_exec_array,
    );
    registry.natives_mut().register(
        "java/lang/Runtime",
        "exec",
        "([Ljava/lang/String;[Ljava/lang/String;Ljava/io/File;)Ljava/lang/Process;",
        native_runtime_exec_array_dir,
    );

    let process_input_stream_ctx = ClassContext {
        class_name: "duke/process/ProcessInputStream".to_string(),
        super_class: Some("java/io/InputStream".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fd".to_string(),
            descriptor: "I".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/lang/AutoCloseable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(process_input_stream_ctx);
    registry.natives_mut().register(
        "duke/process/ProcessInputStream",
        "read",
        "()I",
        native_file_input_stream_read,
    );
    registry.natives_mut().register(
        "duke/process/ProcessInputStream",
        "read",
        "([B)I",
        native_file_input_stream_read_bytes,
    );
    registry.natives_mut().register(
        "duke/process/ProcessInputStream",
        "close",
        "()V",
        native_file_input_stream_close,
    );

    let process_error_stream_ctx = ClassContext {
        class_name: "duke/process/ProcessErrorStream".to_string(),
        super_class: Some("java/io/InputStream".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fd".to_string(),
            descriptor: "I".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/lang/AutoCloseable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(process_error_stream_ctx);
    registry.natives_mut().register(
        "duke/process/ProcessErrorStream",
        "read",
        "()I",
        native_file_input_stream_read,
    );
    registry.natives_mut().register(
        "duke/process/ProcessErrorStream",
        "read",
        "([B)I",
        native_file_input_stream_read_bytes,
    );
    registry.natives_mut().register(
        "duke/process/ProcessErrorStream",
        "close",
        "()V",
        native_file_input_stream_close,
    );

    let process_output_stream_ctx = ClassContext {
        class_name: "duke/process/ProcessOutputStream".to_string(),
        super_class: Some("java/io/OutputStream".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fd".to_string(),
            descriptor: "I".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/lang/AutoCloseable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(process_output_stream_ctx);
    registry.natives_mut().register(
        "duke/process/ProcessOutputStream",
        "write",
        "(I)V",
        native_file_output_stream_write,
    );
    registry.natives_mut().register(
        "duke/process/ProcessOutputStream",
        "write",
        "([B)V",
        native_file_output_stream_write_bytes,
    );
    registry.natives_mut().register(
        "duke/process/ProcessOutputStream",
        "close",
        "()V",
        native_file_output_stream_close,
    );

    let file_input_stream_ctx = ClassContext {
        class_name: "java/io/FileInputStream".to_string(),
        super_class: Some("java/io/InputStream".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fd".to_string(),
            descriptor: "I".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/lang/AutoCloseable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(file_input_stream_ctx);
    registry.natives_mut().register(
        "java/io/FileInputStream",
        "<init>",
        "(Ljava/lang/String;)V",
        native_file_input_stream_init,
    );
    registry.natives_mut().register(
        "java/io/FileInputStream",
        "read",
        "()I",
        native_file_input_stream_read,
    );
    registry.natives_mut().register(
        "java/io/FileInputStream",
        "read",
        "([B)I",
        native_file_input_stream_read_bytes,
    );
    registry.natives_mut().register(
        "java/io/FileInputStream",
        "close",
        "()V",
        native_file_input_stream_close,
    );

    let file_output_stream_ctx = ClassContext {
        class_name: "java/io/FileOutputStream".to_string(),
        super_class: Some("java/io/OutputStream".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fd".to_string(),
            descriptor: "I".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/lang/AutoCloseable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(file_output_stream_ctx);
    registry.natives_mut().register(
        "java/io/FileOutputStream",
        "<init>",
        "(Ljava/lang/String;)V",
        native_file_output_stream_init,
    );
    registry.natives_mut().register(
        "java/io/FileOutputStream",
        "write",
        "(I)V",
        native_file_output_stream_write,
    );
    registry.natives_mut().register(
        "java/io/FileOutputStream",
        "write",
        "([B)V",
        native_file_output_stream_write_bytes,
    );
    registry.natives_mut().register(
        "java/io/FileOutputStream",
        "close",
        "()V",
        native_file_output_stream_close,
    );

    let server_socket_ctx = ClassContext {
        class_name: "java/net/ServerSocket".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "fd".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "port".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/lang/AutoCloseable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(server_socket_ctx);
    registry.natives_mut().register(
        "java/net/ServerSocket",
        "<init>",
        "(I)V",
        native_server_socket_init,
    );
    registry.natives_mut().register(
        "java/net/ServerSocket",
        "accept",
        "()Ljava/net/Socket;",
        native_server_socket_accept,
    );
    registry.natives_mut().register(
        "java/net/ServerSocket",
        "getLocalPort",
        "()I",
        native_server_socket_get_local_port,
    );
    registry.natives_mut().register(
        "java/net/ServerSocket",
        "close",
        "()V",
        native_server_socket_close,
    );

    let socket_ctx = ClassContext {
        class_name: "java/net/Socket".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "fdRead".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "fdWrite".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/lang/AutoCloseable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(socket_ctx);
    registry.natives_mut().register(
        "java/net/Socket",
        "<init>",
        "(Ljava/lang/String;I)V",
        native_socket_init,
    );
    registry.natives_mut().register(
        "java/net/Socket",
        "getInputStream",
        "()Ljava/io/InputStream;",
        native_socket_get_input_stream,
    );
    registry.natives_mut().register(
        "java/net/Socket",
        "getOutputStream",
        "()Ljava/io/OutputStream;",
        native_socket_get_output_stream,
    );
    registry
        .natives_mut()
        .register("java/net/Socket", "close", "()V", native_socket_close);

    let socket_input_stream_ctx = ClassContext {
        class_name: "duke/net/SocketInputStream".to_string(),
        super_class: Some("java/io/InputStream".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fd".to_string(),
            descriptor: "I".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/lang/AutoCloseable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(socket_input_stream_ctx);
    registry.natives_mut().register(
        "duke/net/SocketInputStream",
        "read",
        "()I",
        native_file_input_stream_read,
    );
    registry.natives_mut().register(
        "duke/net/SocketInputStream",
        "read",
        "([B)I",
        native_file_input_stream_read_bytes,
    );
    registry.natives_mut().register(
        "duke/net/SocketInputStream",
        "close",
        "()V",
        native_file_input_stream_close,
    );

    let socket_output_stream_ctx = ClassContext {
        class_name: "duke/net/SocketOutputStream".to_string(),
        super_class: Some("java/io/OutputStream".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fd".to_string(),
            descriptor: "I".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/lang/AutoCloseable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(socket_output_stream_ctx);
    registry.natives_mut().register(
        "duke/net/SocketOutputStream",
        "write",
        "(I)V",
        native_file_output_stream_write,
    );
    registry.natives_mut().register(
        "duke/net/SocketOutputStream",
        "write",
        "([B)V",
        native_file_output_stream_write_bytes,
    );
    registry.natives_mut().register(
        "duke/net/SocketOutputStream",
        "close",
        "()V",
        native_file_output_stream_close,
    );

    // Register java/lang/String ClassContext (empty — instance methods are native).
    let string_ctx = ClassContext {
        class_name: "java/lang/String".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec![
            "java/lang/Comparable".to_string(),
            "java/io/Serializable".to_string(),
            "java/lang/CharSequence".to_string(),
        ],
        bootstrap_methods: Vec::new(),
    };
    registry.register(string_ctx);

    // String instance methods
    registry
        .natives_mut()
        .register("java/lang/String", "length", "()I", native_string_length);
    registry.natives_mut().register(
        "java/lang/String",
        "equals",
        "(Ljava/lang/Object;)Z",
        native_string_equals,
    );
    registry
        .natives_mut()
        .register("java/lang/String", "charAt", "(I)C", native_string_char_at);
    registry.natives_mut().register(
        "java/lang/String",
        "substring",
        "(I)Ljava/lang/String;",
        native_string_substring,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "substring",
        "(II)Ljava/lang/String;",
        native_string_substring_range,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "indexOf",
        "(Ljava/lang/String;)I",
        native_string_indexof,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "contains",
        "(Ljava/lang/CharSequence;)Z",
        native_string_contains,
    );
    registry
        .natives_mut()
        .register("java/lang/String", "isEmpty", "()Z", native_string_isempty);
    registry.natives_mut().register(
        "java/lang/String",
        "compareTo",
        "(Ljava/lang/String;)I",
        native_string_compareto,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "compareTo",
        "(Ljava/lang/Object;)I",
        native_string_compareto_object,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "startsWith",
        "(Ljava/lang/String;)Z",
        native_string_startswith,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "endsWith",
        "(Ljava/lang/String;)Z",
        native_string_endswith,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "trim",
        "()Ljava/lang/String;",
        native_string_trim,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "toCharArray",
        "()[C",
        native_string_tochararray,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "toUpperCase",
        "()Ljava/lang/String;",
        native_string_touppercase,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "toLowerCase",
        "()Ljava/lang/String;",
        native_string_tolowercase,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "replace",
        "(CC)Ljava/lang/String;",
        native_string_replace_char,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "replace",
        "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Ljava/lang/String;",
        native_string_replace_charsequence,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "split",
        "(Ljava/lang/String;)[Ljava/lang/String;",
        native_string_split,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "hashCode",
        "()I",
        native_string_hashcode,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "toString",
        "()Ljava/lang/String;",
        native_string_tostring,
    );

    // String static methods
    registry.natives_mut().register(
        "java/lang/String",
        "valueOf",
        "(I)Ljava/lang/String;",
        native_string_value_of_int,
    );

    // PrintStream.print (no newline)
    registry.natives_mut().register(
        "java/io/PrintStream",
        "print",
        "(Ljava/lang/String;)V",
        native_print_string,
    );
    registry
        .natives_mut()
        .register("java/io/PrintStream", "print", "(I)V", native_print_int);

    // println overloads
    registry.natives_mut().register(
        "java/io/PrintStream",
        "println",
        "(J)V",
        native_println_long,
    );
    registry.natives_mut().register(
        "java/io/PrintStream",
        "println",
        "(F)V",
        native_println_float,
    );
    registry.natives_mut().register(
        "java/io/PrintStream",
        "println",
        "(D)V",
        native_println_double,
    );
    registry.natives_mut().register(
        "java/io/PrintStream",
        "println",
        "(Z)V",
        native_println_boolean,
    );
    registry.natives_mut().register(
        "java/io/PrintStream",
        "println",
        "(C)V",
        native_println_char,
    );
    registry.natives_mut().register(
        "java/io/PrintStream",
        "println",
        "(Ljava/lang/Object;)V",
        native_println_object,
    );

    // print overloads
    registry
        .natives_mut()
        .register("java/io/PrintStream", "print", "(J)V", native_print_long);
    registry
        .natives_mut()
        .register("java/io/PrintStream", "print", "(F)V", native_print_float);
    registry
        .natives_mut()
        .register("java/io/PrintStream", "print", "(D)V", native_print_double);
    registry
        .natives_mut()
        .register("java/io/PrintStream", "print", "(Z)V", native_print_boolean);
    registry
        .natives_mut()
        .register("java/io/PrintStream", "print", "(C)V", native_print_char);
    registry.natives_mut().register(
        "java/io/PrintStream",
        "print",
        "(Ljava/lang/Object;)V",
        native_print_object,
    );

    // System.exit (static)
    registry
        .natives_mut()
        .register("java/lang/System", "exit", "(I)V", native_system_exit);
    registry.natives_mut().register(
        "java/lang/System",
        "currentTimeMillis",
        "()J",
        native_system_current_time_millis,
    );
    registry.natives_mut().register(
        "java/lang/System",
        "nanoTime",
        "()J",
        native_system_nano_time,
    );
    registry.natives_mut().register(
        "java/lang/System",
        "getProperty",
        "(Ljava/lang/String;)Ljava/lang/String;",
        native_system_get_property,
    );
    registry.natives_mut().register(
        "java/lang/System",
        "getProperty",
        "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
        native_system_get_property_with_default,
    );
    registry.natives_mut().register(
        "java/lang/System",
        "setProperty",
        "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
        native_system_set_property,
    );

    // Register synthetic exception hierarchy so is_assignable_from can walk it.
    // java/lang/Object (root — no super)
    let object_ctx = ClassContext {
        class_name: "java/lang/Object".to_string(),
        super_class: None,
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(object_ctx);

    // Object instance methods
    registry
        .natives_mut()
        .register("java/lang/Object", "<init>", "()V", native_object_init);
    registry.natives_mut().register(
        "java/lang/Object",
        "equals",
        "(Ljava/lang/Object;)Z",
        native_object_equals,
    );
    registry.natives_mut().register(
        "java/lang/Object",
        "hashCode",
        "()I",
        native_object_hashcode,
    );
    registry.natives_mut().register(
        "java/lang/Object",
        "toString",
        "()Ljava/lang/String;",
        native_object_tostring,
    );
    registry.natives_mut().register(
        "java/lang/Object",
        "clone",
        "()Ljava/lang/Object;",
        native_object_clone,
    );
    registry.natives_mut().register(
        "java/lang/Object",
        "getClass",
        "()Ljava/lang/Class;",
        native_object_get_class,
    );

    // java/lang/Class — lightweight stub for class literals
    let class_ctx = ClassContext {
        class_name: "java/lang/Class".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(class_ctx);
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "forName",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        native_class_for_name,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "forName",
        "(Ljava/lang/String;ZLjava/lang/ClassLoader;)Ljava/lang/Class;",
        native_class_for_name_with_loader,
    );
    registry.natives_mut().register(
        "java/lang/Class",
        "getName",
        "()Ljava/lang/String;",
        native_class_get_name,
    );
    registry.natives_mut().register(
        "java/lang/Class",
        "getPackageName",
        "()Ljava/lang/String;",
        native_class_get_package_name,
    );
    registry.natives_mut().register(
        "java/lang/Class",
        "desiredAssertionStatus",
        "()Z",
        native_class_desired_assertion_status,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getDeclaredMethods",
        "()[Ljava/lang/reflect/Method;",
        native_class_get_declared_methods,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getDeclaredFields",
        "()[Ljava/lang/reflect/Field;",
        native_class_get_declared_fields,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getDeclaredConstructors",
        "()[Ljava/lang/reflect/Constructor;",
        native_class_get_declared_constructors,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getMethods",
        "()[Ljava/lang/reflect/Method;",
        native_class_get_methods,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getFields",
        "()[Ljava/lang/reflect/Field;",
        native_class_get_fields,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getConstructors",
        "()[Ljava/lang/reflect/Constructor;",
        native_class_get_constructors,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getDeclaredMethod",
        "(Ljava/lang/String;[Ljava/lang/Class;)Ljava/lang/reflect/Method;",
        native_class_get_declared_method,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getDeclaredField",
        "(Ljava/lang/String;)Ljava/lang/reflect/Field;",
        native_class_get_declared_field,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getDeclaredConstructor",
        "([Ljava/lang/Class;)Ljava/lang/reflect/Constructor;",
        native_class_get_declared_constructor,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getMethod",
        "(Ljava/lang/String;[Ljava/lang/Class;)Ljava/lang/reflect/Method;",
        native_class_get_method,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getField",
        "(Ljava/lang/String;)Ljava/lang/reflect/Field;",
        native_class_get_field,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getConstructor",
        "([Ljava/lang/Class;)Ljava/lang/reflect/Constructor;",
        native_class_get_constructor,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "newInstance",
        "()Ljava/lang/Object;",
        native_class_new_instance,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getProtectionDomain",
        "()Ljava/security/ProtectionDomain;",
        native_class_get_protection_domain,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getClassLoader",
        "()Ljava/lang/ClassLoader;",
        native_class_get_class_loader,
    );
    registry.natives_mut().register(
        "jdk/internal/misc/CDS",
        "isDumpingClassList0",
        "()Z",
        native_false_boolean,
    );
    registry.natives_mut().register(
        "jdk/internal/misc/CDS",
        "isDumpingArchive0",
        "()Z",
        native_false_boolean,
    );
    registry.natives_mut().register(
        "jdk/internal/misc/CDS",
        "isSharingEnabled0",
        "()Z",
        native_false_boolean,
    );
    registry.natives_mut().register(
        "jdk/internal/misc/CDS",
        "getRandomSeedForDumping",
        "()J",
        native_zero_long,
    );
    registry.natives_mut().register(
        "jdk/internal/misc/CDS",
        "initializeFromArchive",
        "(Ljava/lang/Class;)V",
        native_void_noop,
    );
    registry.natives_mut().register(
        "jdk/internal/misc/CDS",
        "defineArchivedModules",
        "(Ljava/lang/ClassLoader;Ljava/lang/ClassLoader;)V",
        native_void_noop,
    );
    registry.natives_mut().register(
        "jdk/internal/misc/CDS",
        "logLambdaFormInvoker",
        "(Ljava/lang/String;)V",
        native_void_noop,
    );

    let protection_domain_ctx = ClassContext {
        class_name: "java/security/ProtectionDomain".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "codeSource".to_string(),
            descriptor: "Ljava/security/CodeSource;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(protection_domain_ctx);
    registry.natives_mut().register(
        "java/security/ProtectionDomain",
        "getCodeSource",
        "()Ljava/security/CodeSource;",
        native_protection_domain_get_code_source,
    );

    let code_source_ctx = ClassContext {
        class_name: "java/security/CodeSource".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "location".to_string(),
            descriptor: "Ljava/net/URL;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(code_source_ctx);
    registry.natives_mut().register(
        "java/security/CodeSource",
        "getLocation",
        "()Ljava/net/URL;",
        native_code_source_get_location,
    );

    let url_ctx = ClassContext {
        class_name: "java/net/URL".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "spec".to_string(),
            descriptor: "Ljava/lang/String;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(url_ctx);
    registry.natives_mut().register(
        "java/net/URL",
        "toURI",
        "()Ljava/net/URI;",
        native_url_to_uri,
    );
    registry.natives_mut().register(
        "java/net/URL",
        "setURLStreamHandlerFactory",
        "(Ljava/net/URLStreamHandlerFactory;)V",
        native_url_set_url_stream_handler_factory,
    );

    let uri_ctx = ClassContext {
        class_name: "java/net/URI".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "spec".to_string(),
            descriptor: "Ljava/lang/String;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(uri_ctx);

    let path_ctx = ClassContext {
        class_name: "java/nio/file/Path".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "path".to_string(),
            descriptor: "Ljava/lang/String;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(path_ctx);
    registry.natives_mut().register(
        "java/nio/file/Path",
        "of",
        "(Ljava/net/URI;)Ljava/nio/file/Path;",
        native_path_of,
    );
    registry.natives_mut().register(
        "java/nio/file/Path",
        "toFile",
        "()Ljava/io/File;",
        native_path_to_file,
    );
    let paths_ctx = ClassContext {
        class_name: "java/nio/file/Paths".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(paths_ctx);
    registry.natives_mut().register(
        "java/nio/file/Paths",
        "get",
        "(Ljava/lang/String;[Ljava/lang/String;)Ljava/nio/file/Path;",
        native_paths_get,
    );

    let file_attribute_ctx = ClassContext {
        class_name: "java/nio/file/attribute/FileAttribute".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(file_attribute_ctx);

    let posix_owner_read_ref =
        heap.allocate("java/nio/file/attribute/PosixFilePermission".to_string(), 0);
    let posix_owner_write_ref =
        heap.allocate("java/nio/file/attribute/PosixFilePermission".to_string(), 0);
    let posix_owner_execute_ref =
        heap.allocate("java/nio/file/attribute/PosixFilePermission".to_string(), 0);
    let posix_file_permission_ctx = ClassContext {
        class_name: "java/nio/file/attribute/PosixFilePermission".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "OWNER_READ".to_string(),
                descriptor: "Ljava/nio/file/attribute/PosixFilePermission;".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "OWNER_WRITE".to_string(),
                descriptor: "Ljava/nio/file/attribute/PosixFilePermission;".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "OWNER_EXECUTE".to_string(),
                descriptor: "Ljava/nio/file/attribute/PosixFilePermission;".to_string(),
                is_static: true,
            },
        ],
        static_fields: vec![
            Slot::Reference(Some(posix_owner_read_ref)),
            Slot::Reference(Some(posix_owner_write_ref)),
            Slot::Reference(Some(posix_owner_execute_ref)),
        ],
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(posix_file_permission_ctx);

    let posix_file_permissions_ctx = ClassContext {
        class_name: "java/nio/file/attribute/PosixFilePermissions".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(posix_file_permissions_ctx);
    registry.natives_mut().register(
        "java/nio/file/attribute/PosixFilePermissions",
        "asFileAttribute",
        "(Ljava/util/Set;)Ljava/nio/file/attribute/FileAttribute;",
        native_posix_file_permissions_as_file_attribute,
    );
    let boot_archive_entry_ctx = ClassContext {
        class_name: "duke/boot/ArchiveEntry".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "name".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "directory".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["org/springframework/boot/loader/launch/Archive$Entry".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(boot_archive_entry_ctx);
    registry.natives_mut().register(
        "duke/boot/ArchiveEntry",
        "name",
        "()Ljava/lang/String;",
        native_boot_archive_entry_name,
    );
    registry.natives_mut().register(
        "duke/boot/ArchiveEntry",
        "isDirectory",
        "()Z",
        native_boot_archive_entry_is_directory,
    );
    registry.natives_mut().register_callback(
        "org/springframework/boot/loader/launch/JarFileArchive",
        "getClassPathUrls",
        "(Ljava/util/function/Predicate;Ljava/util/function/Predicate;)Ljava/util/Set;",
        native_boot_jar_file_archive_get_class_path_urls,
    );
    registry.natives_mut().register(
        "org/springframework/boot/loader/launch/Archive",
        "getManifest",
        "()Ljava/util/jar/Manifest;",
        native_boot_archive_get_manifest,
    );
    registry.natives_mut().register(
        "org/springframework/boot/loader/launch/JarFileArchive",
        "getManifest",
        "()Ljava/util/jar/Manifest;",
        native_boot_jar_file_archive_get_manifest,
    );
    registry.natives_mut().register_callback(
        "org/springframework/boot/loader/launch/ExplodedArchive",
        "getClassPathUrls",
        "(Ljava/util/function/Predicate;Ljava/util/function/Predicate;)Ljava/util/Set;",
        native_boot_exploded_archive_get_class_path_urls,
    );
    registry.natives_mut().register(
        "org/springframework/boot/loader/launch/ExplodedArchive",
        "getManifest",
        "()Ljava/util/jar/Manifest;",
        native_boot_exploded_archive_get_manifest,
    );
    registry.natives_mut().register_callback(
        "org/springframework/boot/loader/launch/LaunchedClassLoader",
        "<init>",
        "(ZLorg/springframework/boot/loader/launch/Archive;[Ljava/net/URL;Ljava/lang/ClassLoader;)V",
        native_boot_launched_class_loader_init,
    );

    let reflect_method_ctx = ClassContext {
        class_name: "java/lang/reflect/Method".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "declaringClass".to_string(),
                descriptor: "Ljava/lang/Class;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "name".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "descriptor".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "publicFlag".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "staticFlag".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "accessibleFlag".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 6,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(reflect_method_ctx);
    registry.natives_mut().register(
        "java/lang/reflect/Method",
        "getName",
        "()Ljava/lang/String;",
        native_reflect_method_get_name,
    );
    registry.natives_mut().register(
        "java/lang/reflect/Method",
        "getDeclaringClass",
        "()Ljava/lang/Class;",
        native_reflection_member_get_declaring_class,
    );
    registry.natives_mut().register_callback(
        "java/lang/reflect/Method",
        "getReturnType",
        "()Ljava/lang/Class;",
        native_reflect_method_get_return_type,
    );
    registry.natives_mut().register_callback(
        "java/lang/reflect/Method",
        "invoke",
        "(Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;",
        native_reflect_method_invoke,
    );
    registry.natives_mut().register(
        "java/lang/reflect/Method",
        "setAccessible",
        "(Z)V",
        native_reflection_member_set_accessible,
    );
    registry.natives_mut().register(
        "java/lang/reflect/Method",
        "getParameterCount",
        "()I",
        native_reflect_method_get_parameter_count,
    );
    registry.natives_mut().register_callback(
        "java/lang/reflect/Method",
        "getParameterTypes",
        "()[Ljava/lang/Class;",
        native_reflect_executable_get_parameter_types,
    );

    let reflect_constructor_ctx = ClassContext {
        class_name: "java/lang/reflect/Constructor".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "declaringClass".to_string(),
                descriptor: "Ljava/lang/Class;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "name".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "descriptor".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "publicFlag".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "staticFlag".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "accessibleFlag".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 6,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(reflect_constructor_ctx);
    registry.natives_mut().register(
        "java/lang/reflect/Constructor",
        "getName",
        "()Ljava/lang/String;",
        native_reflect_constructor_get_name,
    );
    registry.natives_mut().register(
        "java/lang/reflect/Constructor",
        "getDeclaringClass",
        "()Ljava/lang/Class;",
        native_reflection_member_get_declaring_class,
    );
    registry.natives_mut().register(
        "java/lang/reflect/Constructor",
        "setAccessible",
        "(Z)V",
        native_reflection_member_set_accessible,
    );
    registry.natives_mut().register(
        "java/lang/reflect/Constructor",
        "getParameterCount",
        "()I",
        native_reflect_method_get_parameter_count,
    );
    registry.natives_mut().register_callback(
        "java/lang/reflect/Constructor",
        "getParameterTypes",
        "()[Ljava/lang/Class;",
        native_reflect_executable_get_parameter_types,
    );
    registry.natives_mut().register_callback(
        "java/lang/reflect/Constructor",
        "newInstance",
        "([Ljava/lang/Object;)Ljava/lang/Object;",
        native_reflect_constructor_new_instance,
    );

    let reflect_field_ctx = ClassContext {
        class_name: "java/lang/reflect/Field".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "declaringClass".to_string(),
                descriptor: "Ljava/lang/Class;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "name".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "descriptor".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "publicFlag".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "staticFlag".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "accessibleFlag".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 6,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(reflect_field_ctx);
    registry.natives_mut().register(
        "java/lang/reflect/Field",
        "getName",
        "()Ljava/lang/String;",
        native_reflect_field_get_name,
    );
    registry.natives_mut().register(
        "java/lang/reflect/Field",
        "getDeclaringClass",
        "()Ljava/lang/Class;",
        native_reflection_member_get_declaring_class,
    );
    registry.natives_mut().register_callback(
        "java/lang/reflect/Field",
        "getType",
        "()Ljava/lang/Class;",
        native_reflect_field_get_type,
    );
    registry.natives_mut().register_callback(
        "java/lang/reflect/Field",
        "get",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_reflect_field_get,
    );
    registry.natives_mut().register_callback(
        "java/lang/reflect/Field",
        "set",
        "(Ljava/lang/Object;Ljava/lang/Object;)V",
        native_reflect_field_set,
    );
    registry.natives_mut().register(
        "java/lang/reflect/Field",
        "setAccessible",
        "(Z)V",
        native_reflection_member_set_accessible,
    );

    let runnable_ctx = ClassContext {
        class_name: "java/lang/Runnable".to_string(),
        super_class: None,
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(runnable_ctx);

    let class_loader_ctx = ClassContext {
        class_name: "java/lang/ClassLoader".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(class_loader_ctx);
    registry.natives_mut().register(
        "java/lang/ClassLoader",
        "registerAsParallelCapable",
        "()Z",
        native_class_loader_register_as_parallel_capable,
    );

    let thread_ctx = ClassContext {
        class_name: "java/lang/Thread".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "target".to_string(),
                descriptor: "Ljava/lang/Runnable;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "threadId".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(thread_ctx);
    registry
        .natives_mut()
        .register("java/lang/Thread", "<init>", "()V", native_thread_init);
    registry.natives_mut().register(
        "java/lang/Thread",
        "<init>",
        "(Ljava/lang/Runnable;)V",
        native_thread_init_runnable,
    );
    registry.natives_mut().register(
        "java/lang/Thread",
        "currentThread",
        "()Ljava/lang/Thread;",
        native_thread_current_thread,
    );
    registry
        .natives_mut()
        .register("java/lang/Thread", "start", "()V", native_thread_start);
    registry
        .natives_mut()
        .register("java/lang/Thread", "join", "()V", native_thread_join);
    registry
        .natives_mut()
        .register("java/lang/Thread", "sleep", "(J)V", native_thread_sleep);
    registry.natives_mut().register(
        "java/lang/Thread",
        "setContextClassLoader",
        "(Ljava/lang/ClassLoader;)V",
        native_void_noop,
    );

    // java/lang/Throwable extends Object
    let throwable_ctx = ClassContext {
        class_name: "java/lang/Throwable".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(throwable_ctx);
    registry.natives_mut().register(
        "java/lang/Throwable",
        "addSuppressed",
        "(Ljava/lang/Throwable;)V",
        native_throwable_add_suppressed,
    );
    registry
        .natives_mut()
        .register("java/lang/Throwable", "<init>", "()V", native_object_init);
    registry.natives_mut().register(
        "java/lang/Throwable",
        "<init>",
        "(Ljava/lang/String;)V",
        native_object_init,
    );

    // java/lang/Exception extends Throwable
    let exception_ctx = ClassContext {
        class_name: "java/lang/Exception".to_string(),
        super_class: Some("java/lang/Throwable".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(exception_ctx);
    registry
        .natives_mut()
        .register("java/lang/Exception", "<init>", "()V", native_object_init);
    registry.natives_mut().register(
        "java/lang/Exception",
        "<init>",
        "(Ljava/lang/String;)V",
        native_object_init,
    );

    // java/lang/RuntimeException extends Exception
    let rte_ctx = ClassContext {
        class_name: "java/lang/RuntimeException".to_string(),
        super_class: Some("java/lang/Exception".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(rte_ctx);
    registry.natives_mut().register(
        "java/lang/RuntimeException",
        "<init>",
        "()V",
        native_object_init,
    );
    registry.natives_mut().register(
        "java/lang/RuntimeException",
        "<init>",
        "(Ljava/lang/String;)V",
        native_object_init,
    );

    let illegal_argument_ctx = ClassContext {
        class_name: "java/lang/IllegalArgumentException".to_string(),
        super_class: Some("java/lang/RuntimeException".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(illegal_argument_ctx);

    let illegal_thread_state_ctx = ClassContext {
        class_name: "java/lang/IllegalThreadStateException".to_string(),
        super_class: Some("java/lang/IllegalArgumentException".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(illegal_thread_state_ctx);

    let reflective_operation_ctx = ClassContext {
        class_name: "java/lang/ReflectiveOperationException".to_string(),
        super_class: Some("java/lang/Exception".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(reflective_operation_ctx);

    let class_not_found_ctx = ClassContext {
        class_name: "java/lang/ClassNotFoundException".to_string(),
        super_class: Some("java/lang/ReflectiveOperationException".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(class_not_found_ctx);

    let no_such_method_ctx = ClassContext {
        class_name: "java/lang/NoSuchMethodException".to_string(),
        super_class: Some("java/lang/ReflectiveOperationException".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(no_such_method_ctx);

    let no_such_field_ctx = ClassContext {
        class_name: "java/lang/NoSuchFieldException".to_string(),
        super_class: Some("java/lang/ReflectiveOperationException".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(no_such_field_ctx);

    let illegal_access_ctx = ClassContext {
        class_name: "java/lang/IllegalAccessException".to_string(),
        super_class: Some("java/lang/ReflectiveOperationException".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(illegal_access_ctx);

    let instantiation_ctx = ClassContext {
        class_name: "java/lang/InstantiationException".to_string(),
        super_class: Some("java/lang/ReflectiveOperationException".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(instantiation_ctx);

    let invocation_target_ctx = ClassContext {
        class_name: "java/lang/reflect/InvocationTargetException".to_string(),
        super_class: Some("java/lang/ReflectiveOperationException".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(invocation_target_ctx);

    // java/lang/AutoCloseable — marker interface for try-with-resources.
    // Registered so is_assignable_from correctly handles queries like
    // "does TryWithResources$Res implement AutoCloseable?" without
    // erroring on an unknown class.
    let autocloseable_ctx = ClassContext {
        class_name: "java/lang/AutoCloseable".to_string(),
        super_class: None, // interface — no super class
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(autocloseable_ctx);

    // java/lang/Enum — abstract superclass for all enums.
    // Fields: name (String) at index 0, ordinal (int) at index 1.
    let enum_ctx = ClassContext {
        class_name: "java/lang/Enum".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "name".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "ordinal".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(enum_ctx);
    registry.natives_mut().register(
        "java/lang/Enum",
        "<init>",
        "(Ljava/lang/String;I)V",
        native_enum_init,
    );
    registry
        .natives_mut()
        .register("java/lang/Enum", "ordinal", "()I", native_enum_ordinal);
    registry.natives_mut().register(
        "java/lang/Enum",
        "name",
        "()Ljava/lang/String;",
        native_enum_name,
    );
    registry.natives_mut().register(
        "java/lang/Enum",
        "valueOf",
        "(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;",
        native_enum_valueof,
    );

    // java/lang/Integer — boxed int with value field + numeric constants
    let integer_ctx = ClassContext {
        class_name: "java/lang/Integer".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "value".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "MAX_VALUE".to_string(),
                descriptor: "I".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "MIN_VALUE".to_string(),
                descriptor: "I".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "TYPE".to_string(),
                descriptor: "Ljava/lang/Class;".to_string(),
                is_static: true,
            },
        ],
        static_fields: vec![
            Slot::Int(i32::MAX),
            Slot::Int(i32::MIN),
            Slot::Reference(None),
        ],
        instance_field_count: 1,
        interfaces: vec!["java/lang/Comparable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(integer_ctx);
    registry.natives_mut().register(
        "java/lang/Integer",
        "parseInt",
        "(Ljava/lang/String;)I",
        native_integer_parseint,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "parseInt",
        "(Ljava/lang/String;I)I",
        native_integer_parseint_radix,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "valueOf",
        "(Ljava/lang/String;)Ljava/lang/Integer;",
        native_integer_valueof_string,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "valueOf",
        "(Ljava/lang/String;I)Ljava/lang/Integer;",
        native_integer_valueof_string_radix,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "decode",
        "(Ljava/lang/String;)Ljava/lang/Integer;",
        native_integer_decode,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "valueOf",
        "(I)Ljava/lang/Integer;",
        native_integer_valueof,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "intValue",
        "()I",
        native_integer_intvalue,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "toString",
        "(I)Ljava/lang/String;",
        native_integer_tostring_static,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "toHexString",
        "(I)Ljava/lang/String;",
        native_integer_tohexstring_static,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "toOctalString",
        "(I)Ljava/lang/String;",
        native_integer_tooctalstring_static,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "toBinaryString",
        "(I)Ljava/lang/String;",
        native_integer_tobinarystring_static,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "toUnsignedLong",
        "(I)J",
        native_integer_tounsignedlong_static,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "compareUnsigned",
        "(II)I",
        native_integer_compareunsigned_static,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "compareTo",
        "(Ljava/lang/Object;)I",
        native_integer_compareto,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "compareTo",
        "(Ljava/lang/Integer;)I",
        native_integer_compareto,
    );

    // java/lang/Long — boxed long with value field + numeric constants
    let long_ctx = ClassContext {
        class_name: "java/lang/Long".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "value".to_string(),
                descriptor: "J".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "MAX_VALUE".to_string(),
                descriptor: "J".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "MIN_VALUE".to_string(),
                descriptor: "J".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "TYPE".to_string(),
                descriptor: "Ljava/lang/Class;".to_string(),
                is_static: true,
            },
        ],
        static_fields: vec![
            Slot::Long(i64::MAX),
            Slot::Long(i64::MIN),
            Slot::Reference(None),
        ],
        instance_field_count: 1,
        interfaces: vec!["java/lang/Comparable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(long_ctx);
    registry.natives_mut().register(
        "java/lang/Long",
        "parseLong",
        "(Ljava/lang/String;)J",
        native_long_parselong,
    );
    registry.natives_mut().register(
        "java/lang/Long",
        "parseLong",
        "(Ljava/lang/String;I)J",
        native_long_parselong_radix,
    );
    registry.natives_mut().register(
        "java/lang/Long",
        "valueOf",
        "(Ljava/lang/String;)Ljava/lang/Long;",
        native_long_valueof_string,
    );
    registry.natives_mut().register(
        "java/lang/Long",
        "valueOf",
        "(Ljava/lang/String;I)Ljava/lang/Long;",
        native_long_valueof_string_radix,
    );
    registry.natives_mut().register(
        "java/lang/Long",
        "decode",
        "(Ljava/lang/String;)Ljava/lang/Long;",
        native_long_decode,
    );
    registry.natives_mut().register(
        "java/lang/Long",
        "valueOf",
        "(J)Ljava/lang/Long;",
        native_long_valueof,
    );
    registry
        .natives_mut()
        .register("java/lang/Long", "longValue", "()J", native_long_longvalue);
    registry.natives_mut().register(
        "java/lang/Long",
        "toString",
        "(J)Ljava/lang/String;",
        native_long_tostring_static,
    );
    registry.natives_mut().register(
        "java/lang/Long",
        "toHexString",
        "(J)Ljava/lang/String;",
        native_long_tohexstring_static,
    );
    registry.natives_mut().register(
        "java/lang/Long",
        "toOctalString",
        "(J)Ljava/lang/String;",
        native_long_tooctalstring_static,
    );
    registry.natives_mut().register(
        "java/lang/Long",
        "toBinaryString",
        "(J)Ljava/lang/String;",
        native_long_tobinarystring_static,
    );
    registry.natives_mut().register(
        "java/lang/Long",
        "compareUnsigned",
        "(JJ)I",
        native_long_compareunsigned_static,
    );
    registry.natives_mut().register(
        "java/lang/Long",
        "compareTo",
        "(Ljava/lang/Object;)I",
        native_long_compareto,
    );
    registry.natives_mut().register(
        "java/lang/Long",
        "compareTo",
        "(Ljava/lang/Long;)I",
        native_long_compareto,
    );

    // java/lang/Double — boxed double with value field + numeric constants
    let double_ctx = ClassContext {
        class_name: "java/lang/Double".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "value".to_string(),
                descriptor: "D".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "MAX_VALUE".to_string(),
                descriptor: "D".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "MIN_VALUE".to_string(),
                descriptor: "D".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "NaN".to_string(),
                descriptor: "D".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "POSITIVE_INFINITY".to_string(),
                descriptor: "D".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "NEGATIVE_INFINITY".to_string(),
                descriptor: "D".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "TYPE".to_string(),
                descriptor: "Ljava/lang/Class;".to_string(),
                is_static: true,
            },
        ],
        static_fields: vec![
            Slot::Double(f64::MAX),
            Slot::Double(5e-324_f64),
            Slot::Double(f64::NAN),
            Slot::Double(f64::INFINITY),
            Slot::Double(f64::NEG_INFINITY),
            Slot::Reference(None),
        ],
        instance_field_count: 1,
        interfaces: vec!["java/lang/Comparable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(double_ctx);
    registry.natives_mut().register(
        "java/lang/Double",
        "parseDouble",
        "(Ljava/lang/String;)D",
        native_double_parsedouble,
    );
    registry.natives_mut().register(
        "java/lang/Double",
        "valueOf",
        "(D)Ljava/lang/Double;",
        native_double_valueof,
    );
    registry.natives_mut().register(
        "java/lang/Double",
        "doubleValue",
        "()D",
        native_double_doublevalue,
    );
    registry
        .natives_mut()
        .register("java/lang/Double", "isNaN", "(D)Z", native_double_isnan);
    registry.natives_mut().register(
        "java/lang/Double",
        "compareTo",
        "(Ljava/lang/Object;)I",
        native_double_compareto,
    );
    registry.natives_mut().register(
        "java/lang/Double",
        "compareTo",
        "(Ljava/lang/Double;)I",
        native_double_compareto,
    );

    // java/lang/Float — boxed float with value field
    let float_ctx = ClassContext {
        class_name: "java/lang/Float".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "value".to_string(),
                descriptor: "F".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "TYPE".to_string(),
                descriptor: "Ljava/lang/Class;".to_string(),
                is_static: true,
            },
        ],
        static_fields: vec![Slot::Reference(None)],
        instance_field_count: 1,
        interfaces: vec!["java/lang/Comparable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(float_ctx);
    registry.natives_mut().register(
        "java/lang/Float",
        "parseFloat",
        "(Ljava/lang/String;)F",
        native_float_parsefloat,
    );
    registry.natives_mut().register(
        "java/lang/Float",
        "valueOf",
        "(F)Ljava/lang/Float;",
        native_float_valueof,
    );
    registry.natives_mut().register(
        "java/lang/Float",
        "floatValue",
        "()F",
        native_float_floatvalue,
    );
    registry.natives_mut().register(
        "java/lang/Float",
        "compareTo",
        "(Ljava/lang/Object;)I",
        native_float_compareto,
    );
    registry.natives_mut().register(
        "java/lang/Float",
        "compareTo",
        "(Ljava/lang/Float;)I",
        native_float_compareto,
    );

    // java/lang/Boolean — boxed boolean + static utility
    let boolean_ctx = ClassContext {
        class_name: "java/lang/Boolean".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "value".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "TYPE".to_string(),
                descriptor: "Ljava/lang/Class;".to_string(),
                is_static: true,
            },
        ],
        static_fields: vec![Slot::Reference(None)],
        instance_field_count: 1,
        interfaces: vec!["java/lang/Comparable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(boolean_ctx);
    registry.natives_mut().register(
        "java/lang/Boolean",
        "parseBoolean",
        "(Ljava/lang/String;)Z",
        native_boolean_parseboolean,
    );
    registry.natives_mut().register(
        "java/lang/Boolean",
        "valueOf",
        "(Z)Ljava/lang/Boolean;",
        native_boolean_valueof,
    );
    registry.natives_mut().register(
        "java/lang/Boolean",
        "booleanValue",
        "()Z",
        native_boolean_booleanvalue,
    );
    registry.natives_mut().register(
        "java/lang/Boolean",
        "compareTo",
        "(Ljava/lang/Object;)I",
        native_boolean_compareto,
    );
    registry.natives_mut().register(
        "java/lang/Boolean",
        "compareTo",
        "(Ljava/lang/Boolean;)I",
        native_boolean_compareto,
    );

    let byte_ctx = ClassContext {
        class_name: "java/lang/Byte".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "value".to_string(),
                descriptor: "B".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "TYPE".to_string(),
                descriptor: "Ljava/lang/Class;".to_string(),
                is_static: true,
            },
        ],
        static_fields: vec![Slot::Reference(None)],
        instance_field_count: 1,
        interfaces: vec!["java/lang/Comparable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(byte_ctx);
    registry.natives_mut().register(
        "java/lang/Byte",
        "parseByte",
        "(Ljava/lang/String;)B",
        native_byte_parsebyte,
    );
    registry.natives_mut().register(
        "java/lang/Byte",
        "parseByte",
        "(Ljava/lang/String;I)B",
        native_byte_parsebyte_radix,
    );
    registry.natives_mut().register(
        "java/lang/Byte",
        "valueOf",
        "(Ljava/lang/String;)Ljava/lang/Byte;",
        native_byte_valueof_string,
    );
    registry.natives_mut().register(
        "java/lang/Byte",
        "valueOf",
        "(Ljava/lang/String;I)Ljava/lang/Byte;",
        native_byte_valueof_string_radix,
    );
    registry.natives_mut().register(
        "java/lang/Byte",
        "valueOf",
        "(B)Ljava/lang/Byte;",
        native_byte_valueof,
    );
    registry
        .natives_mut()
        .register("java/lang/Byte", "byteValue", "()B", native_byte_bytevalue);
    registry.natives_mut().register(
        "java/lang/Byte",
        "compareTo",
        "(Ljava/lang/Object;)I",
        native_byte_compareto,
    );
    registry.natives_mut().register(
        "java/lang/Byte",
        "compareTo",
        "(Ljava/lang/Byte;)I",
        native_byte_compareto,
    );

    let short_ctx = ClassContext {
        class_name: "java/lang/Short".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "value".to_string(),
                descriptor: "S".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "TYPE".to_string(),
                descriptor: "Ljava/lang/Class;".to_string(),
                is_static: true,
            },
        ],
        static_fields: vec![Slot::Reference(None)],
        instance_field_count: 1,
        interfaces: vec!["java/lang/Comparable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(short_ctx);
    registry.natives_mut().register(
        "java/lang/Short",
        "parseShort",
        "(Ljava/lang/String;)S",
        native_short_parseshort,
    );
    registry.natives_mut().register(
        "java/lang/Short",
        "parseShort",
        "(Ljava/lang/String;I)S",
        native_short_parseshort_radix,
    );
    registry.natives_mut().register(
        "java/lang/Short",
        "valueOf",
        "(Ljava/lang/String;)Ljava/lang/Short;",
        native_short_valueof_string,
    );
    registry.natives_mut().register(
        "java/lang/Short",
        "valueOf",
        "(Ljava/lang/String;I)Ljava/lang/Short;",
        native_short_valueof_string_radix,
    );
    registry.natives_mut().register(
        "java/lang/Short",
        "valueOf",
        "(S)Ljava/lang/Short;",
        native_short_valueof,
    );
    registry.natives_mut().register(
        "java/lang/Short",
        "shortValue",
        "()S",
        native_short_shortvalue,
    );
    registry.natives_mut().register(
        "java/lang/Short",
        "compareTo",
        "(Ljava/lang/Object;)I",
        native_short_compareto,
    );
    registry.natives_mut().register(
        "java/lang/Short",
        "compareTo",
        "(Ljava/lang/Short;)I",
        native_short_compareto,
    );

    let void_ctx = ClassContext {
        class_name: "java/lang/Void".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "TYPE".to_string(),
            descriptor: "Ljava/lang/Class;".to_string(),
            is_static: true,
        }],
        static_fields: vec![Slot::Reference(None)],
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(void_ctx);

    // java/lang/Math — static math utilities with PI and E constants
    let math_ctx = ClassContext {
        class_name: "java/lang/Math".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "PI".to_string(),
                descriptor: "D".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "E".to_string(),
                descriptor: "D".to_string(),
                is_static: true,
            },
        ],
        static_fields: vec![
            Slot::Double(std::f64::consts::PI),
            Slot::Double(std::f64::consts::E),
        ],
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(math_ctx);
    registry
        .natives_mut()
        .register("java/lang/Math", "max", "(II)I", native_math_max_int);
    registry
        .natives_mut()
        .register("java/lang/Math", "min", "(II)I", native_math_min_int);
    registry
        .natives_mut()
        .register("java/lang/Math", "abs", "(I)I", native_math_abs_int);
    registry.natives_mut().register(
        "java/lang/Math",
        "floorMod",
        "(II)I",
        native_math_floor_mod_int,
    );
    registry
        .natives_mut()
        .register("java/lang/Math", "sqrt", "(D)D", native_math_sqrt);
    registry
        .natives_mut()
        .register("java/lang/Math", "pow", "(DD)D", native_math_pow);
    registry
        .natives_mut()
        .register("java/lang/Math", "floor", "(D)D", native_math_floor);
    registry
        .natives_mut()
        .register("java/lang/Math", "ceil", "(D)D", native_math_ceil);
    registry
        .natives_mut()
        .register("java/lang/Math", "round", "(D)J", native_math_round_double);
    registry
        .natives_mut()
        .register("java/lang/Math", "abs", "(J)J", native_math_abs_long);
    registry
        .natives_mut()
        .register("java/lang/Math", "abs", "(D)D", native_math_abs_double);
    registry
        .natives_mut()
        .register("java/lang/Math", "max", "(JJ)J", native_math_max_long);
    registry
        .natives_mut()
        .register("java/lang/Math", "min", "(JJ)J", native_math_min_long);
    registry
        .natives_mut()
        .register("java/lang/Math", "max", "(DD)D", native_math_max_double);
    registry
        .natives_mut()
        .register("java/lang/Math", "min", "(DD)D", native_math_min_double);

    // String.valueOf overloads (int already registered above)
    registry.natives_mut().register(
        "java/lang/String",
        "valueOf",
        "(J)Ljava/lang/String;",
        native_string_value_of_long,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "valueOf",
        "(D)Ljava/lang/String;",
        native_string_value_of_double,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "valueOf",
        "(F)Ljava/lang/String;",
        native_string_value_of_float,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "valueOf",
        "(Z)Ljava/lang/String;",
        native_string_value_of_boolean,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "valueOf",
        "(C)Ljava/lang/String;",
        native_string_value_of_char,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "valueOf",
        "(Ljava/lang/Object;)Ljava/lang/String;",
        native_string_value_of_object,
    );

    // String.concat
    registry.natives_mut().register(
        "java/lang/String",
        "concat",
        "(Ljava/lang/String;)Ljava/lang/String;",
        native_string_concat,
    );

    // String.format(String, Object[]) -> String
    registry.natives_mut().register(
        "java/lang/String",
        "format",
        "(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/String;",
        native_string_format,
    );

    // java/lang/StringBuilder — mutable string buffer
    let sb_ctx = ClassContext {
        class_name: "java/lang/StringBuilder".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/lang/CharSequence".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(sb_ctx);

    // StringBuilder.<init>()V
    registry
        .natives_mut()
        .register("java/lang/StringBuilder", "<init>", "()V", native_sb_init);
    // StringBuilder.<init>(String)V
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "<init>",
        "(Ljava/lang/String;)V",
        native_sb_init_string,
    );
    // StringBuilder.append(String)
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "append",
        "(Ljava/lang/String;)Ljava/lang/StringBuilder;",
        native_sb_append_string,
    );
    // StringBuilder.append(int)
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "append",
        "(I)Ljava/lang/StringBuilder;",
        native_sb_append_int,
    );
    // StringBuilder.append(long)
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "append",
        "(J)Ljava/lang/StringBuilder;",
        native_sb_append_long,
    );
    // StringBuilder.append(double)
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "append",
        "(D)Ljava/lang/StringBuilder;",
        native_sb_append_double,
    );
    // StringBuilder.append(float)
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "append",
        "(F)Ljava/lang/StringBuilder;",
        native_sb_append_float,
    );
    // StringBuilder.append(boolean)
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "append",
        "(Z)Ljava/lang/StringBuilder;",
        native_sb_append_boolean,
    );
    // StringBuilder.append(char)
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "append",
        "(C)Ljava/lang/StringBuilder;",
        native_sb_append_char,
    );
    // StringBuilder.toString()
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "toString",
        "()Ljava/lang/String;",
        native_sb_tostring,
    );
    // StringBuilder.length()
    registry
        .natives_mut()
        .register("java/lang/StringBuilder", "length", "()I", native_sb_length);

    // java/lang/Character — static character utilities + boxed char
    let character_ctx = ClassContext {
        class_name: "java/lang/Character".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "value".to_string(),
                descriptor: "C".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "TYPE".to_string(),
                descriptor: "Ljava/lang/Class;".to_string(),
                is_static: true,
            },
        ],
        static_fields: vec![Slot::Reference(None)],
        instance_field_count: 1,
        interfaces: vec!["java/lang/Comparable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(character_ctx);

    // Character.isDigit(C)Z
    registry.natives_mut().register(
        "java/lang/Character",
        "isDigit",
        "(C)Z",
        native_char_is_digit,
    );
    // Character.isLetter(C)Z
    registry.natives_mut().register(
        "java/lang/Character",
        "isLetter",
        "(C)Z",
        native_char_is_letter,
    );
    // Character.isWhitespace(C)Z
    registry.natives_mut().register(
        "java/lang/Character",
        "isWhitespace",
        "(C)Z",
        native_char_is_whitespace,
    );
    // Character.isUpperCase(C)Z
    registry.natives_mut().register(
        "java/lang/Character",
        "isUpperCase",
        "(C)Z",
        native_char_is_uppercase,
    );
    // Character.isLowerCase(C)Z
    registry.natives_mut().register(
        "java/lang/Character",
        "isLowerCase",
        "(C)Z",
        native_char_is_lowercase,
    );
    // Character.toUpperCase(C)C
    registry.natives_mut().register(
        "java/lang/Character",
        "toUpperCase",
        "(C)C",
        native_char_to_uppercase,
    );
    // Character.toLowerCase(C)C
    registry.natives_mut().register(
        "java/lang/Character",
        "toLowerCase",
        "(C)C",
        native_char_to_lowercase,
    );
    // Character.isLetterOrDigit(C)Z
    registry.natives_mut().register(
        "java/lang/Character",
        "isLetterOrDigit",
        "(C)Z",
        native_char_is_letter_or_digit,
    );
    // Character.valueOf(C)Ljava/lang/Character;
    registry.natives_mut().register(
        "java/lang/Character",
        "valueOf",
        "(C)Ljava/lang/Character;",
        native_char_valueof,
    );
    // Character.charValue()C
    registry.natives_mut().register(
        "java/lang/Character",
        "charValue",
        "()C",
        native_char_charvalue,
    );
    registry.natives_mut().register(
        "java/lang/Character",
        "compareTo",
        "(Ljava/lang/Object;)I",
        native_char_compareto,
    );
    registry.natives_mut().register(
        "java/lang/Character",
        "compareTo",
        "(Ljava/lang/Character;)I",
        native_char_compareto,
    );

    let collection_ctx = ClassContext {
        class_name: "java/util/Collection".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(collection_ctx);
    registry.natives_mut().register(
        "java/util/Collection",
        "toArray",
        "()[Ljava/lang/Object;",
        native_collection_to_array,
    );
    registry.natives_mut().register(
        "java/util/Collection",
        "toArray",
        "([Ljava/lang/Object;)[Ljava/lang/Object;",
        native_collection_to_array_with_seed_array,
    );

    // java/util/ArrayList — dynamic list backed by growable fields
    // fields[0] = size (Int), fields[1..] = elements (pushed dynamically)
    let arraylist_ctx = ClassContext {
        class_name: "java/util/ArrayList".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "size".to_string(),
            descriptor: "I".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec![
            "java/util/List".to_string(),
            "java/util/Collection".to_string(),
            "java/lang/Iterable".to_string(),
        ],
        bootstrap_methods: Vec::new(),
    };
    registry.register(arraylist_ctx);
    registry.natives_mut().register(
        "java/util/ArrayList",
        "<init>",
        "()V",
        native_arraylist_init,
    );
    registry.natives_mut().register(
        "java/util/ArrayList",
        "add",
        "(Ljava/lang/Object;)Z",
        native_arraylist_add,
    );
    registry.natives_mut().register(
        "java/util/ArrayList",
        "get",
        "(I)Ljava/lang/Object;",
        native_arraylist_get,
    );
    registry
        .natives_mut()
        .register("java/util/ArrayList", "size", "()I", native_arraylist_size);
    registry.natives_mut().register(
        "java/util/ArrayList",
        "toArray",
        "()[Ljava/lang/Object;",
        native_collection_to_array,
    );
    registry.natives_mut().register(
        "java/util/ArrayList",
        "toArray",
        "([Ljava/lang/Object;)[Ljava/lang/Object;",
        native_collection_to_array_with_seed_array,
    );
    registry.natives_mut().register(
        "java/util/ArrayList",
        "iterator",
        "()Ljava/util/Iterator;",
        native_arraylist_iterator,
    );
    registry.natives_mut().register_callback(
        "java/util/ArrayList",
        "sort",
        "(Ljava/util/Comparator;)V",
        array_list_sort,
    );

    // duke/util/ArrayListIterator — internal iterator for ArrayList
    // fields[0] = ArrayList reference, fields[1] = current index (Int)
    let iter_ctx = ClassContext {
        class_name: "duke/util/ArrayListIterator".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "list".to_string(),
                descriptor: "Ljava/util/ArrayList;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "index".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(iter_ctx);
    registry.natives_mut().register(
        "duke/util/ArrayListIterator",
        "<init>",
        "()V",
        native_arraylist_iter_init,
    );
    registry.natives_mut().register(
        "duke/util/ArrayListIterator",
        "hasNext",
        "()Z",
        native_arraylist_iter_hasnext,
    );
    registry.natives_mut().register(
        "duke/util/ArrayListIterator",
        "next",
        "()Ljava/lang/Object;",
        native_arraylist_iter_next,
    );

    // java/util/Arrays — static array utilities
    let arrays_ctx = ClassContext {
        class_name: "java/util/Arrays".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(arrays_ctx);
    registry
        .natives_mut()
        .register("java/util/Arrays", "fill", "([II)V", native_arrays_fill_int);
    registry.natives_mut().register(
        "java/util/Arrays",
        "fill",
        "([Ljava/lang/Object;Ljava/lang/Object;)V",
        native_arrays_fill_object,
    );
    registry.natives_mut().register(
        "java/util/Arrays",
        "copyOf",
        "([II)[I",
        native_arrays_copyof_int,
    );
    registry.natives_mut().register(
        "java/util/Arrays",
        "copyOf",
        "([Ljava/lang/Object;I)[Ljava/lang/Object;",
        native_arrays_copyof_object,
    );
    registry
        .natives_mut()
        .register("java/util/Arrays", "sort", "([I)V", native_arrays_sort_int);

    // java/util/HashMap — hash map backed by flat key/value pair list in fields
    // fields[0] = Int(size), fields[1]=key0, fields[2]=val0, fields[3]=key1, ...
    let hashmap_ctx = ClassContext {
        class_name: "java/util/HashMap".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "size".to_string(),
            descriptor: "I".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(hashmap_ctx);
    registry
        .natives_mut()
        .register("java/util/HashMap", "<init>", "()V", native_hashmap_init);
    registry.natives_mut().register(
        "java/util/HashMap",
        "put",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        native_hashmap_put,
    );
    registry.natives_mut().register(
        "java/util/HashMap",
        "get",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_hashmap_get,
    );
    registry.natives_mut().register(
        "java/util/HashMap",
        "containsKey",
        "(Ljava/lang/Object;)Z",
        native_hashmap_contains_key,
    );
    registry
        .natives_mut()
        .register("java/util/HashMap", "size", "()I", native_hashmap_size);
    registry.natives_mut().register(
        "java/util/HashMap",
        "remove",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_hashmap_remove,
    );
    registry.natives_mut().register(
        "java/util/HashMap",
        "isEmpty",
        "()Z",
        native_hashmap_is_empty,
    );
    registry.natives_mut().register(
        "java/util/HashMap",
        "getOrDefault",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        native_hashmap_get_or_default,
    );

    // java/util/HashSet — set backed by unique elements in fields
    // fields[0] = Int(size), fields[1..] = elements (unique, no duplicates)
    let hashset_ctx = ClassContext {
        class_name: "java/util/HashSet".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "size".to_string(),
            descriptor: "I".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec![
            "java/util/Set".to_string(),
            "java/util/Collection".to_string(),
            "java/lang/Iterable".to_string(),
        ],
        bootstrap_methods: Vec::new(),
    };
    let set_ctx = ClassContext {
        class_name: "java/util/Set".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(set_ctx);
    registry.natives_mut().register(
        "java/util/Set",
        "of",
        "([Ljava/lang/Object;)Ljava/util/Set;",
        native_set_of,
    );
    registry.register(hashset_ctx);
    registry
        .natives_mut()
        .register("java/util/HashSet", "<init>", "()V", native_hashset_init);
    registry.natives_mut().register_callback(
        "java/util/HashSet",
        "<init>",
        "(Ljava/util/Collection;)V",
        native_hashset_init_from_collection,
    );
    registry.natives_mut().register(
        "java/util/HashSet",
        "add",
        "(Ljava/lang/Object;)Z",
        native_hashset_add,
    );
    registry.natives_mut().register(
        "java/util/HashSet",
        "contains",
        "(Ljava/lang/Object;)Z",
        native_hashset_contains,
    );
    registry.natives_mut().register(
        "java/util/HashSet",
        "remove",
        "(Ljava/lang/Object;)Z",
        native_hashset_remove,
    );
    registry
        .natives_mut()
        .register("java/util/HashSet", "size", "()I", native_hashset_size);
    registry.natives_mut().register(
        "java/util/HashSet",
        "isEmpty",
        "()Z",
        native_hashset_is_empty,
    );
    registry.natives_mut().register(
        "java/util/HashSet",
        "iterator",
        "()Ljava/util/Iterator;",
        native_hashset_iterator,
    );
    registry.natives_mut().register(
        "java/util/HashSet",
        "toArray",
        "()[Ljava/lang/Object;",
        native_collection_to_array,
    );
    registry.natives_mut().register(
        "java/util/HashSet",
        "toArray",
        "([Ljava/lang/Object;)[Ljava/lang/Object;",
        native_collection_to_array_with_seed_array,
    );

    let hashset_iter_ctx = ClassContext {
        class_name: "duke/util/HashSetIterator".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "set".to_string(),
                descriptor: "Ljava/util/HashSet;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "index".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/Iterator".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(hashset_iter_ctx);
    registry.natives_mut().register(
        "duke/util/HashSetIterator",
        "<init>",
        "()V",
        native_hashset_iter_init,
    );
    registry.natives_mut().register(
        "duke/util/HashSetIterator",
        "hasNext",
        "()Z",
        native_hashset_iter_hasnext,
    );
    registry.natives_mut().register(
        "duke/util/HashSetIterator",
        "next",
        "()Ljava/lang/Object;",
        native_hashset_iter_next,
    );

    // java/util/Collections — static utility class
    // Only sort(List) is supported; delegates to ArrayList.sort(null).
    let collections_ctx = ClassContext {
        class_name: "java/util/Collections".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(collections_ctx);
    registry.natives_mut().register_callback(
        "java/util/Collections",
        "sort",
        "(Ljava/util/List;)V",
        native_collections_sort,
    );

    // ── java.util.zip ──────────────────────────────────────────────────

    let zip_exception_ctx = ClassContext {
        class_name: "java/util/zip/ZipException".to_string(),
        super_class: Some("java/io/IOException".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(zip_exception_ctx);

    // ZipEntry: [name: Ref, compressedSize_lo: Int, compressedSize_hi: Int,
    //            size_lo: Int, size_hi: Int, method: Int]  → 6 fields
    let zip_entry_ctx = ClassContext {
        class_name: "java/util/zip/ZipEntry".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "name".into(),
                descriptor: "Ljava/lang/String;".into(),
                is_static: false,
            },
            FieldEntry {
                name: "compressedSize_lo".into(),
                descriptor: "I".into(),
                is_static: false,
            },
            FieldEntry {
                name: "compressedSize_hi".into(),
                descriptor: "I".into(),
                is_static: false,
            },
            FieldEntry {
                name: "size_lo".into(),
                descriptor: "I".into(),
                is_static: false,
            },
            FieldEntry {
                name: "size_hi".into(),
                descriptor: "I".into(),
                is_static: false,
            },
            FieldEntry {
                name: "method".into(),
                descriptor: "I".into(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 6,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(zip_entry_ctx);
    registry.natives_mut().register(
        "java/util/zip/ZipEntry",
        "getName",
        "()Ljava/lang/String;",
        native_zip_entry_get_name,
    );
    registry.natives_mut().register(
        "java/util/zip/ZipEntry",
        "getCompressedSize",
        "()J",
        native_zip_entry_get_compressed_size,
    );
    registry.natives_mut().register(
        "java/util/zip/ZipEntry",
        "getSize",
        "()J",
        native_zip_entry_get_size,
    );
    registry.natives_mut().register(
        "java/util/zip/ZipEntry",
        "getMethod",
        "()I",
        native_zip_entry_get_method,
    );

    // ZipFile: [fd: Int] → 1 field
    let zip_file_ctx = ClassContext {
        class_name: "java/util/zip/ZipFile".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fd".into(),
            descriptor: "I".into(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/lang/AutoCloseable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(zip_file_ctx);
    registry.natives_mut().register(
        "java/util/zip/ZipFile",
        "<init>",
        "(Ljava/lang/String;)V",
        native_zip_file_init,
    );
    registry.natives_mut().register(
        "java/util/zip/ZipFile",
        "getEntry",
        "(Ljava/lang/String;)Ljava/util/zip/ZipEntry;",
        native_zip_file_get_entry,
    );
    registry.natives_mut().register(
        "java/util/zip/ZipFile",
        "getInputStream",
        "(Ljava/util/zip/ZipEntry;)Ljava/io/InputStream;",
        native_zip_file_get_input_stream,
    );
    registry.natives_mut().register(
        "java/util/zip/ZipFile",
        "close",
        "()V",
        native_zip_file_close,
    );
    registry
        .natives_mut()
        .register("java/util/zip/ZipFile", "size", "()I", native_zip_file_size);

    // JarFile extends ZipFile — inherits everything for now.
    let jar_file_ctx = ClassContext {
        class_name: "java/util/jar/JarFile".to_string(),
        super_class: Some("java/util/zip/ZipFile".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(jar_file_ctx);
    registry.natives_mut().register(
        "java/util/jar/JarFile",
        "<init>",
        "(Ljava/io/File;)V",
        native_jar_file_init_from_file,
    );
    registry.natives_mut().register(
        "java/util/jar/JarFile",
        "<init>",
        "(Ljava/io/File;ZILjava/lang/Runtime$Version;)V",
        native_jar_file_init_with_mode_and_version,
    );
    registry.natives_mut().register(
        "java/util/jar/JarFile",
        "getManifest",
        "()Ljava/util/jar/Manifest;",
        native_jar_file_get_manifest,
    );
    registry.natives_mut().register(
        "org/springframework/boot/loader/jar/NestedJarFile",
        "getManifest",
        "()Ljava/util/jar/Manifest;",
        native_boot_nested_jar_file_get_manifest,
    );
    registry.natives_mut().register(
        "org/springframework/boot/loader/net/protocol/jar/UrlJarFile",
        "getManifest",
        "()Ljava/util/jar/Manifest;",
        native_boot_nested_jar_file_get_manifest,
    );
    registry.natives_mut().register(
        "org/springframework/boot/loader/net/protocol/jar/UrlNestedJarFile",
        "getManifest",
        "()Ljava/util/jar/Manifest;",
        native_boot_nested_jar_file_get_manifest,
    );

    let manifest_ctx = ClassContext {
        class_name: "java/util/jar/Manifest".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "raw".to_string(),
            descriptor: "Ljava/lang/String;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(manifest_ctx);
    registry.natives_mut().register(
        "java/util/jar/Manifest",
        "getMainAttributes",
        "()Ljava/util/jar/Attributes;",
        native_manifest_get_main_attributes,
    );

    let attributes_ctx = ClassContext {
        class_name: "java/util/jar/Attributes".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "raw".to_string(),
            descriptor: "Ljava/lang/String;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(attributes_ctx);
    registry.natives_mut().register(
        "java/util/jar/Attributes",
        "getValue",
        "(Ljava/lang/String;)Ljava/lang/String;",
        native_attributes_get_value,
    );

    // ByteBufferInputStream — internal class for reading decompressed ZIP data.
    let byte_buffer_is_ctx = ClassContext {
        class_name: "duke/zip/ByteBufferInputStream".to_string(),
        super_class: Some("java/io/InputStream".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fd".into(),
            descriptor: "I".into(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/lang/AutoCloseable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(byte_buffer_is_ctx);
    registry.natives_mut().register(
        "duke/zip/ByteBufferInputStream",
        "read",
        "()I",
        native_file_input_stream_read,
    );
    registry.natives_mut().register(
        "duke/zip/ByteBufferInputStream",
        "read",
        "([B)I",
        native_file_input_stream_read_bytes,
    );
    registry.natives_mut().register(
        "duke/zip/ByteBufferInputStream",
        "close",
        "()V",
        native_file_input_stream_close,
    );
}
