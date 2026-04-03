use crate::context::{ClassContext, FieldEntry};
use crate::registry::ClassRegistry;
#[allow(clippy::wildcard_imports)]
use crate::*;
use duke_runtime::Slot;

#[allow(clippy::too_many_lines)]
pub fn bootstrap_stdlib(registry: &mut ClassRegistry, heap: &mut duke_gc::Heap) {
    // Allocate PrintStream objects for System.out and System.err.
    let ps_out_ref = heap.allocate("java/io/PrintStream".to_string(), 0);
    let ps_err_ref = heap.allocate("java/io/PrintStream".to_string(), 0);

    // Create java/lang/System ClassContext with static fields `out`, `err`, `lineSeparator`.
    let system_ctx = ClassContext {
        class_name: "java/lang/System".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "out".to_string(),
                descriptor: "Ljava/io/PrintStream;".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "err".to_string(),
                descriptor: "Ljava/io/PrintStream;".to_string(),
                is_static: true,
            },
        ],
        static_fields: vec![
            Slot::Reference(Some(ps_out_ref)),
            Slot::Reference(Some(ps_err_ref)),
        ],
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
    registry.natives_mut().register(
        "java/lang/System",
        "arraycopy",
        "(Ljava/lang/Object;ILjava/lang/Object;II)V",
        native_system_arraycopy,
    );
    registry.natives_mut().register(
        "java/lang/System",
        "lineSeparator",
        "()Ljava/lang/String;",
        native_system_line_separator,
    );
    registry.natives_mut().register(
        "java/lang/System",
        "identityHashCode",
        "(Ljava/lang/Object;)I",
        native_system_identity_hash_code,
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
        native_throwable_init_string,
    );
    registry.natives_mut().register(
        "java/lang/Throwable",
        "getMessage",
        "()Ljava/lang/String;",
        native_throwable_get_message,
    );
    registry.natives_mut().register(
        "java/lang/Throwable",
        "toString",
        "()Ljava/lang/String;",
        native_throwable_tostring,
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
        native_throwable_init_string,
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
        native_throwable_init_string,
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
    registry.natives_mut().register(
        "java/lang/Integer",
        "bitCount",
        "(I)I",
        native_integer_bitcount,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "numberOfLeadingZeros",
        "(I)I",
        native_integer_leading_zeros,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "numberOfTrailingZeros",
        "(I)I",
        native_integer_trailing_zeros,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "highestOneBit",
        "(I)I",
        native_integer_highest_one_bit,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "lowestOneBit",
        "(I)I",
        native_integer_lowest_one_bit,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "reverse",
        "(I)I",
        native_integer_reverse,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "reverseBytes",
        "(I)I",
        native_integer_reverse_bytes,
    );
    registry
        .natives_mut()
        .register("java/lang/Integer", "signum", "(I)I", native_integer_signum);
    registry.natives_mut().register(
        "java/lang/Integer",
        "compare",
        "(II)I",
        native_integer_compare_static,
    );
    registry
        .natives_mut()
        .register("java/lang/Integer", "sum", "(II)I", native_integer_sum);
    registry.natives_mut().register(
        "java/lang/Integer",
        "max",
        "(II)I",
        native_integer_max_static,
    );
    registry.natives_mut().register(
        "java/lang/Integer",
        "min",
        "(II)I",
        native_integer_min_static,
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
    registry
        .natives_mut()
        .register("java/lang/Long", "bitCount", "(J)I", native_long_bitcount);
    registry.natives_mut().register(
        "java/lang/Long",
        "numberOfLeadingZeros",
        "(J)I",
        native_long_leading_zeros,
    );
    registry.natives_mut().register(
        "java/lang/Long",
        "numberOfTrailingZeros",
        "(J)I",
        native_long_trailing_zeros,
    );
    registry.natives_mut().register(
        "java/lang/Long",
        "highestOneBit",
        "(J)J",
        native_long_highest_one_bit,
    );
    registry.natives_mut().register(
        "java/lang/Long",
        "lowestOneBit",
        "(J)J",
        native_long_lowest_one_bit,
    );
    registry
        .natives_mut()
        .register("java/lang/Long", "reverse", "(J)J", native_long_reverse);
    registry.natives_mut().register(
        "java/lang/Long",
        "reverseBytes",
        "(J)J",
        native_long_reverse_bytes,
    );
    registry
        .natives_mut()
        .register("java/lang/Long", "signum", "(J)I", native_long_signum);
    registry.natives_mut().register(
        "java/lang/Long",
        "compare",
        "(JJ)I",
        native_long_compare_static,
    );
    registry
        .natives_mut()
        .register("java/lang/Long", "sum", "(JJ)J", native_long_sum);

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
    // Allocate TRUE/FALSE singletons before registering the class so static_fields
    // can reference them via heap address.
    let bool_true_ref = heap.allocate("java/lang/Boolean".to_string(), 1);
    if let Ok(obj) = heap.get_mut(bool_true_ref) {
        obj.fields[0] = Slot::Int(1);
    }
    let bool_false_ref = heap.allocate("java/lang/Boolean".to_string(), 1);
    if let Ok(obj) = heap.get_mut(bool_false_ref) {
        obj.fields[0] = Slot::Int(0);
    }
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
                name: "TRUE".to_string(),
                descriptor: "Ljava/lang/Boolean;".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "FALSE".to_string(),
                descriptor: "Ljava/lang/Boolean;".to_string(),
                is_static: true,
            },
            FieldEntry {
                name: "TYPE".to_string(),
                descriptor: "Ljava/lang/Class;".to_string(),
                is_static: true,
            },
        ],
        static_fields: vec![
            Slot::Reference(Some(bool_true_ref)),
            Slot::Reference(Some(bool_false_ref)),
            Slot::Reference(None), // TYPE
        ],
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
    registry
        .natives_mut()
        .register("java/lang/Math", "sin", "(D)D", native_math_sin);
    registry
        .natives_mut()
        .register("java/lang/Math", "cos", "(D)D", native_math_cos);
    registry
        .natives_mut()
        .register("java/lang/Math", "tan", "(D)D", native_math_tan);
    registry
        .natives_mut()
        .register("java/lang/Math", "asin", "(D)D", native_math_asin);
    registry
        .natives_mut()
        .register("java/lang/Math", "acos", "(D)D", native_math_acos);
    registry
        .natives_mut()
        .register("java/lang/Math", "atan", "(D)D", native_math_atan);
    registry
        .natives_mut()
        .register("java/lang/Math", "atan2", "(DD)D", native_math_atan2);
    registry
        .natives_mut()
        .register("java/lang/Math", "log", "(D)D", native_math_log);
    registry
        .natives_mut()
        .register("java/lang/Math", "log10", "(D)D", native_math_log10);
    registry
        .natives_mut()
        .register("java/lang/Math", "exp", "(D)D", native_math_exp);
    registry.natives_mut().register(
        "java/lang/Math",
        "signum",
        "(D)D",
        native_math_signum_double,
    );
    registry
        .natives_mut()
        .register("java/lang/Math", "signum", "(F)F", native_math_signum_float);
    registry.natives_mut().register(
        "java/lang/Math",
        "toRadians",
        "(D)D",
        native_math_to_radians,
    );
    registry.natives_mut().register(
        "java/lang/Math",
        "toDegrees",
        "(D)D",
        native_math_to_degrees,
    );
    registry
        .natives_mut()
        .register("java/lang/Math", "cbrt", "(D)D", native_math_cbrt);
    registry
        .natives_mut()
        .register("java/lang/Math", "hypot", "(DD)D", native_math_hypot);
    registry.natives_mut().register(
        "java/lang/Math",
        "floorDiv",
        "(II)I",
        native_math_floor_div_int,
    );
    registry
        .natives_mut()
        .register("java/lang/Math", "round", "(F)I", native_math_round_float);

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
    registry.natives_mut().register(
        "java/lang/String",
        "strip",
        "()Ljava/lang/String;",
        native_string_strip,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "stripLeading",
        "()Ljava/lang/String;",
        native_string_strip_leading,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "stripTrailing",
        "()Ljava/lang/String;",
        native_string_strip_trailing,
    );
    registry
        .natives_mut()
        .register("java/lang/String", "isBlank", "()Z", native_string_is_blank);
    registry.natives_mut().register(
        "java/lang/String",
        "repeat",
        "(I)Ljava/lang/String;",
        native_string_repeat,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "formatted",
        "([Ljava/lang/Object;)Ljava/lang/String;",
        native_string_formatted,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "join",
        "(Ljava/lang/CharSequence;[Ljava/lang/CharSequence;)Ljava/lang/String;",
        native_string_join,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "indexOf",
        "(I)I",
        native_string_index_of_char,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "lastIndexOf",
        "(Ljava/lang/String;)I",
        native_string_last_index_of,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "codePointAt",
        "(I)I",
        native_string_code_point_at,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "lines",
        "()Ljava/util/stream/Stream;",
        native_string_lines,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "matches",
        "(Ljava/lang/String;)Z",
        native_string_matches_regex,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "replaceAll",
        "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
        native_string_replace_all_regex,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "replaceFirst",
        "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
        native_string_replace_first_regex,
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
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "insert",
        "(ILjava/lang/String;)Ljava/lang/StringBuilder;",
        native_sb_insert_string,
    );
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "insert",
        "(IC)Ljava/lang/StringBuilder;",
        native_sb_insert_char,
    );
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "delete",
        "(II)Ljava/lang/StringBuilder;",
        native_sb_delete,
    );
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "deleteCharAt",
        "(I)Ljava/lang/StringBuilder;",
        native_sb_delete_char_at,
    );
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "reverse",
        "()Ljava/lang/StringBuilder;",
        native_sb_reverse,
    );
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "charAt",
        "(I)C",
        native_sb_char_at,
    );
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "setLength",
        "(I)V",
        native_sb_set_length,
    );

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
        "<init>",
        "(Ljava/util/Collection;)V",
        native_arraylist_init_from_collection,
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
    registry.natives_mut().register(
        "java/util/ArrayList",
        "remove",
        "(I)Ljava/lang/Object;",
        native_arraylist_remove_at,
    );
    registry.natives_mut().register(
        "java/util/ArrayList",
        "remove",
        "(Ljava/lang/Object;)Z",
        native_arraylist_remove_obj,
    );
    registry.natives_mut().register(
        "java/util/ArrayList",
        "contains",
        "(Ljava/lang/Object;)Z",
        native_arraylist_contains,
    );
    registry.natives_mut().register(
        "java/util/ArrayList",
        "clear",
        "()V",
        native_arraylist_clear,
    );
    registry.natives_mut().register(
        "java/util/ArrayList",
        "isEmpty",
        "()Z",
        native_arraylist_is_empty,
    );
    registry.natives_mut().register(
        "java/util/ArrayList",
        "set",
        "(ILjava/lang/Object;)Ljava/lang/Object;",
        native_arraylist_set,
    );
    registry.natives_mut().register(
        "java/util/ArrayList",
        "indexOf",
        "(Ljava/lang/Object;)I",
        native_arraylist_index_of,
    );
    registry.natives_mut().register(
        "java/util/ArrayList",
        "add",
        "(ILjava/lang/Object;)V",
        native_arraylist_add_at,
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
    registry.natives_mut().register(
        "java/util/Arrays",
        "asList",
        "([Ljava/lang/Object;)Ljava/util/List;",
        native_arrays_as_list,
    );

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
        interfaces: vec![
            "java/util/Map".to_string(),
            "java/util/Collection".to_string(),
        ],
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
    registry.natives_mut().register(
        "java/util/HashMap",
        "keySet",
        "()Ljava/util/Set;",
        native_hashmap_key_set,
    );
    registry.natives_mut().register(
        "java/util/HashMap",
        "values",
        "()Ljava/util/Collection;",
        native_hashmap_values,
    );
    registry.natives_mut().register(
        "java/util/HashMap",
        "entrySet",
        "()Ljava/util/Set;",
        native_hashmap_entry_set,
    );
    registry.natives_mut().register(
        "java/util/HashMap",
        "putIfAbsent",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        native_hashmap_put_if_absent,
    );
    registry
        .natives_mut()
        .register("java/util/HashMap", "clear", "()V", native_hashmap_clear);
    registry.natives_mut().register(
        "java/util/HashMap",
        "containsValue",
        "(Ljava/lang/Object;)Z",
        native_hashmap_contains_value,
    );
    registry.natives_mut().register_callback(
        "java/util/HashMap",
        "forEach",
        "(Ljava/util/function/BiConsumer;)V",
        native_hashmap_for_each,
    );

    // java/util/LinkedList — doubly-ended list/deque backed by ArrayList field layout
    // fields[0] = Int(size), fields[1..] = elements (head-to-tail order)
    // NOTE: super_class is Object (not ArrayList) so instance_field_count is not summed twice.
    let linked_list_ctx = ClassContext {
        class_name: "java/util/LinkedList".to_string(),
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
        interfaces: vec!["java/lang/Iterable".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(linked_list_ctx);
    registry.natives_mut().register(
        "java/util/LinkedList",
        "<init>",
        "()V",
        native_linked_list_init,
    );
    registry.natives_mut().register(
        "java/util/LinkedList",
        "size",
        "()I",
        native_linked_list_size,
    );
    registry.natives_mut().register(
        "java/util/LinkedList",
        "add",
        "(Ljava/lang/Object;)Z",
        native_linked_list_add,
    );
    registry.natives_mut().register(
        "java/util/LinkedList",
        "get",
        "(I)Ljava/lang/Object;",
        native_linked_list_get,
    );
    registry.natives_mut().register(
        "java/util/LinkedList",
        "addFirst",
        "(Ljava/lang/Object;)V",
        native_linked_list_add_first,
    );
    registry.natives_mut().register(
        "java/util/LinkedList",
        "addLast",
        "(Ljava/lang/Object;)V",
        native_linked_list_add_last,
    );
    registry.natives_mut().register(
        "java/util/LinkedList",
        "peekFirst",
        "()Ljava/lang/Object;",
        native_linked_list_peek_first,
    );
    registry.natives_mut().register(
        "java/util/LinkedList",
        "peekLast",
        "()Ljava/lang/Object;",
        native_linked_list_peek_last,
    );
    registry.natives_mut().register(
        "java/util/LinkedList",
        "removeFirst",
        "()Ljava/lang/Object;",
        native_linked_list_remove_first,
    );
    registry.natives_mut().register(
        "java/util/LinkedList",
        "removeLast",
        "()Ljava/lang/Object;",
        native_linked_list_remove_last,
    );
    registry.natives_mut().register(
        "java/util/LinkedList",
        "poll",
        "()Ljava/lang/Object;",
        native_linked_list_poll,
    );
    registry.natives_mut().register(
        "java/util/LinkedList",
        "offer",
        "(Ljava/lang/Object;)Z",
        native_linked_list_offer,
    );
    registry.natives_mut().register(
        "java/util/LinkedList",
        "isEmpty",
        "()Z",
        native_linked_list_is_empty,
    );
    registry.natives_mut().register(
        "java/util/LinkedList",
        "iterator",
        "()Ljava/util/Iterator;",
        native_linked_list_iterator,
    );

    // java/util/Map$Entry — key/value pair produced by HashMap.entrySet()
    // fields[0] = key, fields[1] = value
    let map_entry_ctx = ClassContext {
        class_name: "java/util/Map$Entry".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "key".to_string(),
                descriptor: "Ljava/lang/Object;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "value".to_string(),
                descriptor: "Ljava/lang/Object;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(map_entry_ctx);
    registry.natives_mut().register(
        "java/util/Map$Entry",
        "getKey",
        "()Ljava/lang/Object;",
        native_map_entry_get_key,
    );
    registry.natives_mut().register(
        "java/util/Map$Entry",
        "getValue",
        "()Ljava/lang/Object;",
        native_map_entry_get_value,
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
    registry.natives_mut().register(
        "java/util/Collections",
        "emptyList",
        "()Ljava/util/List;",
        native_collections_empty_list,
    );
    registry.natives_mut().register(
        "java/util/Collections",
        "emptySet",
        "()Ljava/util/Set;",
        native_collections_empty_set,
    );
    registry.natives_mut().register(
        "java/util/Collections",
        "emptyMap",
        "()Ljava/util/Map;",
        native_collections_empty_map,
    );
    registry.natives_mut().register(
        "java/util/Collections",
        "singletonList",
        "(Ljava/lang/Object;)Ljava/util/List;",
        native_collections_singleton_list,
    );
    registry.natives_mut().register(
        "java/util/Collections",
        "reverse",
        "(Ljava/util/List;)V",
        native_collections_reverse,
    );
    registry.natives_mut().register(
        "java/util/Collections",
        "frequency",
        "(Ljava/util/Collection;Ljava/lang/Object;)I",
        native_collections_frequency,
    );
    registry.natives_mut().register_callback(
        "java/util/Collections",
        "sort",
        "(Ljava/util/List;Ljava/util/Comparator;)V",
        native_collections_sort_with_comparator,
    );
    registry.natives_mut().register_callback(
        "java/util/Collections",
        "min",
        "(Ljava/util/Collection;)Ljava/lang/Object;",
        native_collections_min,
    );
    registry.natives_mut().register_callback(
        "java/util/Collections",
        "max",
        "(Ljava/util/Collection;)Ljava/lang/Object;",
        native_collections_max,
    );
    registry.natives_mut().register(
        "java/util/Collections",
        "shuffle",
        "(Ljava/util/List;)V",
        native_collections_shuffle,
    );
    registry.natives_mut().register(
        "java/util/Collections",
        "nCopies",
        "(ILjava/lang/Object;)Ljava/util/List;",
        native_collections_n_copies,
    );
    registry.natives_mut().register(
        "java/util/Collections",
        "unmodifiableList",
        "(Ljava/util/List;)Ljava/util/List;",
        native_collections_unmodifiable_list,
    );

    // HashSet.stream() and LinkedList.stream()
    registry.natives_mut().register(
        "java/util/HashSet",
        "stream",
        "()Ljava/util/stream/Stream;",
        native_hashset_stream,
    );
    registry.natives_mut().register(
        "java/util/LinkedList",
        "stream",
        "()Ljava/util/stream/Stream;",
        native_linked_list_stream,
    );

    // String.chars() → IntStream
    registry.natives_mut().register(
        "java/lang/String",
        "chars",
        "()Ljava/util/stream/IntStream;",
        native_string_chars,
    );
    // String.join(CharSequence, Iterable) overload
    registry.natives_mut().register(
        "java/lang/String",
        "join",
        "(Ljava/lang/CharSequence;Ljava/lang/Iterable;)Ljava/lang/String;",
        native_string_join,
    );

    // Stream.peek and Stream.toArray
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "peek",
        "(Ljava/util/function/Consumer;)Ljava/util/stream/Stream;",
        native_stream_peek,
    );
    registry.natives_mut().register(
        "duke/util/Stream",
        "toArray",
        "()[Ljava/lang/Object;",
        native_stream_to_array,
    );

    // Arrays.stream(int[]) → IntStream
    registry.natives_mut().register(
        "java/util/Arrays",
        "stream",
        "([I)Ljava/util/stream/IntStream;",
        native_arrays_stream_int,
    );
    // Arrays.stream(int[], int, int) → IntStream (subrange)
    registry.natives_mut().register(
        "java/util/Arrays",
        "stream",
        "([III)Ljava/util/stream/IntStream;",
        native_arrays_stream_int_range,
    );
    // Arrays.stream(Object[]) → Stream
    registry.natives_mut().register(
        "java/util/Arrays",
        "stream",
        "([Ljava/lang/Object;)Ljava/util/stream/Stream;",
        native_arrays_stream_object,
    );

    // Math.random()
    registry
        .natives_mut()
        .register("java/lang/Math", "random", "()D", native_math_random);

    // Comparator.comparing(Function)
    registry.natives_mut().register(
        "java/util/Comparator",
        "comparing",
        "(Ljava/util/function/Function;)Ljava/util/Comparator;",
        native_comparator_comparing,
    );
    // duke/util/ComparingComparator — compare(OO)I
    let comparing_comp_ctx = ClassContext {
        class_name: "duke/util/ComparingComparator".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fn".to_string(),
            descriptor: "Ljava/util/function/Function;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/Comparator".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(comparing_comp_ctx);
    registry.natives_mut().register_callback(
        "duke/util/ComparingComparator",
        "compare",
        "(Ljava/lang/Object;Ljava/lang/Object;)I",
        native_comparing_comparator_compare,
    );

    // Collectors.counting() and Collectors.groupingBy()
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "counting",
        "()Ljava/util/stream/Collector;",
        native_collectors_counting,
    );
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "groupingBy",
        "(Ljava/util/function/Function;)Ljava/util/stream/Collector;",
        native_collectors_grouping_by,
    );
    // duke/util/CountingCollector and GroupingByCollector sentinel classes
    let counting_ctx = ClassContext {
        class_name: "duke/util/CountingCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(counting_ctx);
    let grouping_ctx = ClassContext {
        class_name: "duke/util/GroupingByCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fn".to_string(),
            descriptor: "Ljava/util/function/Function;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(grouping_ctx);

    // Collectors.toSet() and Collectors.toMap(keyFn, valFn)
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "toSet",
        "()Ljava/util/stream/Collector;",
        native_collectors_to_set,
    );
    registry.natives_mut().register_callback(
        "java/util/stream/Collectors",
        "toMap",
        "(Ljava/util/function/Function;Ljava/util/function/Function;)Ljava/util/stream/Collector;",
        native_collectors_to_map,
    );
    let to_set_ctx = ClassContext {
        class_name: "duke/util/ToSetCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(to_set_ctx);
    let to_map_ctx = ClassContext {
        class_name: "duke/util/ToMapCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "keyFn".to_string(),
                descriptor: "Ljava/util/function/Function;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "valFn".to_string(),
                descriptor: "Ljava/util/function/Function;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(to_map_ctx);

    // Stream.mapToInt(ToIntFunction) → IntStream
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "mapToInt",
        "(Ljava/util/function/ToIntFunction;)Ljava/util/stream/IntStream;",
        native_stream_map_to_int,
    );

    // IntStream.reduce
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "reduce",
        "(ILjava/util/function/IntBinaryOperator;)I",
        native_int_stream_reduce_identity,
    );
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "reduce",
        "(Ljava/util/function/IntBinaryOperator;)Ljava/util/OptionalInt;",
        native_int_stream_reduce_optional,
    );

    // Stream.reduce(BinaryOperator) and Stream.reduce(identity, BinaryOperator) — Phase 37 had reduce, but adding 2-arg
    // Stream.min(Comparator) and Stream.max(Comparator)
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "min",
        "(Ljava/util/Comparator;)Ljava/util/Optional;",
        native_stream_min_comparator,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "max",
        "(Ljava/util/Comparator;)Ljava/util/Optional;",
        native_stream_max_comparator,
    );

    // Arrays.sort(Object[]) — natural order sort
    registry.natives_mut().register_callback(
        "java/util/Arrays",
        "sort",
        "([Ljava/lang/Object;)V",
        native_arrays_sort_objects,
    );

    // Arrays.copyOfRange (int[] and Object[] variants)
    registry.natives_mut().register(
        "java/util/Arrays",
        "copyOfRange",
        "([III)[I",
        native_arrays_copy_of_range_int,
    );
    registry.natives_mut().register(
        "java/util/Arrays",
        "copyOfRange",
        "([Ljava/lang/Object;II)[Ljava/lang/Object;",
        native_arrays_copy_of_range_object,
    );

    // String.<init>(String)V — copy constructor
    registry.natives_mut().register(
        "java/lang/String",
        "<init>",
        "(Ljava/lang/String;)V",
        native_string_init_copy,
    );
    // String.<init>(char[]) and String.valueOf(char[])
    registry.natives_mut().register(
        "java/lang/String",
        "<init>",
        "([C)V",
        native_string_init_from_chars,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "valueOf",
        "([C)Ljava/lang/String;",
        native_string_value_of_char_array,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "intern",
        "()Ljava/lang/String;",
        native_string_intern,
    );

    // ArrayList.subList(int, int) → returns a new ArrayList view (copy)
    registry.natives_mut().register(
        "java/util/ArrayList",
        "subList",
        "(II)Ljava/util/List;",
        native_arraylist_sub_list,
    );
    registry.natives_mut().register_callback(
        "java/util/ArrayList",
        "removeIf",
        "(Ljava/util/function/Predicate;)Z",
        native_arraylist_remove_if,
    );

    // List.forEach(Consumer) / ArrayList.forEach(Consumer)
    registry.natives_mut().register_callback(
        "java/util/ArrayList",
        "forEach",
        "(Ljava/util/function/Consumer;)V",
        native_arraylist_for_each,
    );

    // Stream.takeWhile / dropWhile (Java 9)
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "takeWhile",
        "(Ljava/util/function/Predicate;)Ljava/util/stream/Stream;",
        native_stream_take_while,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "dropWhile",
        "(Ljava/util/function/Predicate;)Ljava/util/stream/Stream;",
        native_stream_drop_while,
    );

    // Stream.sorted(Comparator) with comparator
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "sorted",
        "(Ljava/util/Comparator;)Ljava/util/stream/Stream;",
        native_stream_sorted_comparator,
    );

    // Arrays.toString(int[]) and Arrays.toString(Object[])
    registry.natives_mut().register(
        "java/util/Arrays",
        "toString",
        "([I)Ljava/lang/String;",
        native_arrays_to_string_int,
    );
    registry.natives_mut().register(
        "java/util/Arrays",
        "toString",
        "([Ljava/lang/Object;)Ljava/lang/String;",
        native_arrays_to_string_object,
    );

    // HashMap.replace(k, v) → old value or null
    registry.natives_mut().register(
        "java/util/HashMap",
        "replace",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        native_hashmap_replace,
    );

    // Collections.swap(list, i, j)
    registry.natives_mut().register(
        "java/util/Collections",
        "swap",
        "(Ljava/util/List;II)V",
        native_collections_swap,
    );

    // Collections.unmodifiableMap(map) — identity stub
    registry.natives_mut().register(
        "java/util/Collections",
        "unmodifiableMap",
        "(Ljava/util/Map;)Ljava/util/Map;",
        native_collections_unmodifiable_map,
    );
    // Collections.reverseOrder() — same as Comparator.reverseOrder()
    registry.natives_mut().register(
        "java/util/Collections",
        "reverseOrder",
        "()Ljava/util/Comparator;",
        native_comparator_reverse_order,
    );

    // Collectors.partitioningBy(Predicate) — bool-keyed map
    registry.natives_mut().register_callback(
        "java/util/stream/Collectors",
        "partitioningBy",
        "(Ljava/util/function/Predicate;)Ljava/util/stream/Collector;",
        native_collectors_partitioning_by,
    );
    let partitioning_ctx = ClassContext {
        class_name: "duke/util/PartitioningByCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "predicate".to_string(),
            descriptor: "Ljava/util/function/Predicate;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(partitioning_ctx);

    // IntStream.sorted()
    registry.natives_mut().register(
        "duke/util/IntStream",
        "sorted",
        "()Ljava/util/stream/IntStream;",
        native_int_stream_sorted,
    );

    // Comparator.comparingInt(ToIntFunction)
    // (already registered in Phase 34, but adding alias for lambda dispatch)

    // Comparator.reversed() → wraps the comparator in a ReverseComparator
    registry.natives_mut().register_callback(
        "duke/util/ComparingComparator",
        "reversed",
        "()Ljava/util/Comparator;",
        native_comparator_reversed,
    );
    registry.natives_mut().register_callback(
        "duke/util/NaturalOrderComparator",
        "reversed",
        "()Ljava/util/Comparator;",
        native_comparator_reversed,
    );
    let reversed_ctx = ClassContext {
        class_name: "duke/util/ReversedComparator".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "delegate".to_string(),
            descriptor: "Ljava/util/Comparator;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/Comparator".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(reversed_ctx);
    registry.natives_mut().register_callback(
        "duke/util/ReversedComparator",
        "compare",
        "(Ljava/lang/Object;Ljava/lang/Object;)I",
        native_reversed_comparator_compare,
    );

    // Collections.binarySearch
    registry.natives_mut().register_callback(
        "java/util/Collections",
        "binarySearch",
        "(Ljava/util/List;Ljava/lang/Object;)I",
        native_collections_binary_search,
    );

    // java/util/TreeMap — sorted map backed by flat sorted key/val pairs
    // fields[0] = Int(size), fields[1,2] = k0/v0, fields[3,4] = k1/v1, ...
    let treemap_ctx = ClassContext {
        class_name: "java/util/TreeMap".to_string(),
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
    registry.register(treemap_ctx);
    registry
        .natives_mut()
        .register("java/util/TreeMap", "<init>", "()V", native_treemap_init);
    registry.natives_mut().register(
        "java/util/TreeMap",
        "put",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        native_treemap_put,
    );
    registry.natives_mut().register(
        "java/util/TreeMap",
        "get",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_treemap_get,
    );
    registry.natives_mut().register(
        "java/util/TreeMap",
        "containsKey",
        "(Ljava/lang/Object;)Z",
        native_treemap_contains_key,
    );
    registry
        .natives_mut()
        .register("java/util/TreeMap", "size", "()I", native_treemap_size);
    registry.natives_mut().register(
        "java/util/TreeMap",
        "firstKey",
        "()Ljava/lang/Object;",
        native_treemap_first_key,
    );
    registry.natives_mut().register(
        "java/util/TreeMap",
        "lastKey",
        "()Ljava/lang/Object;",
        native_treemap_last_key,
    );
    registry.natives_mut().register(
        "java/util/TreeMap",
        "remove",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_treemap_remove,
    );
    registry.natives_mut().register(
        "java/util/TreeMap",
        "isEmpty",
        "()Z",
        native_treemap_is_empty,
    );

    // java/util/Stack — LIFO stack backed by ArrayList field layout
    let stack_ctx = ClassContext {
        class_name: "java/util/Stack".to_string(),
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
    registry.register(stack_ctx);
    registry
        .natives_mut()
        .register("java/util/Stack", "<init>", "()V", native_stack_init);
    registry.natives_mut().register(
        "java/util/Stack",
        "push",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_stack_push,
    );
    registry.natives_mut().register(
        "java/util/Stack",
        "pop",
        "()Ljava/lang/Object;",
        native_stack_pop,
    );
    registry.natives_mut().register(
        "java/util/Stack",
        "peek",
        "()Ljava/lang/Object;",
        native_stack_peek,
    );
    registry
        .natives_mut()
        .register("java/util/Stack", "empty", "()Z", native_stack_empty);
    registry
        .natives_mut()
        .register("java/util/Stack", "size", "()I", native_stack_size);

    // java/util/Comparator — static factory methods
    let comparator_ctx = ClassContext {
        class_name: "java/util/Comparator".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(comparator_ctx);
    registry.natives_mut().register(
        "java/util/Comparator",
        "naturalOrder",
        "()Ljava/util/Comparator;",
        native_comparator_natural_order,
    );
    registry.natives_mut().register(
        "java/util/Comparator",
        "reverseOrder",
        "()Ljava/util/Comparator;",
        native_comparator_reverse_order,
    );
    registry.natives_mut().register(
        "java/util/Comparator",
        "comparingInt",
        "(Ljava/util/function/ToIntFunction;)Ljava/util/Comparator;",
        native_comparator_comparing_int,
    );

    // duke/util/NaturalOrderComparator — singleton, compare via compareTo
    let natural_ord_ctx = ClassContext {
        class_name: "duke/util/NaturalOrderComparator".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/util/Comparator".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(natural_ord_ctx);
    registry.natives_mut().register_callback(
        "duke/util/NaturalOrderComparator",
        "compare",
        "(Ljava/lang/Object;Ljava/lang/Object;)I",
        native_natural_order_compare,
    );

    // duke/util/ReverseOrderComparator — singleton, negates natural order
    let reverse_ord_ctx = ClassContext {
        class_name: "duke/util/ReverseOrderComparator".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/util/Comparator".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(reverse_ord_ctx);
    registry.natives_mut().register_callback(
        "duke/util/ReverseOrderComparator",
        "compare",
        "(Ljava/lang/Object;Ljava/lang/Object;)I",
        native_reverse_order_compare,
    );

    // duke/util/ComparingIntComparator — wraps a ToIntFunction key extractor
    // fields[0] = Reference(fn_ref)
    let comparing_int_ctx = ClassContext {
        class_name: "duke/util/ComparingIntComparator".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "keyExtractor".to_string(),
            descriptor: "Ljava/util/function/ToIntFunction;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/Comparator".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(comparing_int_ctx);
    registry.natives_mut().register_callback(
        "duke/util/ComparingIntComparator",
        "compare",
        "(Ljava/lang/Object;Ljava/lang/Object;)I",
        native_comparing_int_compare,
    );

    // java/util/TreeSet — sorted set backed by flat sorted elements
    // fields[0] = Int(size), fields[1..] = sorted elements (no duplicates)
    let treeset_ctx = ClassContext {
        class_name: "java/util/TreeSet".to_string(),
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
        interfaces: vec!["java/util/Set".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(treeset_ctx);
    registry
        .natives_mut()
        .register("java/util/TreeSet", "<init>", "()V", native_treeset_init);
    registry.natives_mut().register(
        "java/util/TreeSet",
        "add",
        "(Ljava/lang/Object;)Z",
        native_treeset_add,
    );
    registry.natives_mut().register(
        "java/util/TreeSet",
        "contains",
        "(Ljava/lang/Object;)Z",
        native_treeset_contains,
    );
    registry
        .natives_mut()
        .register("java/util/TreeSet", "size", "()I", native_treeset_size);
    registry.natives_mut().register(
        "java/util/TreeSet",
        "first",
        "()Ljava/lang/Object;",
        native_treeset_first,
    );
    registry.natives_mut().register(
        "java/util/TreeSet",
        "last",
        "()Ljava/lang/Object;",
        native_treeset_last,
    );
    registry.natives_mut().register(
        "java/util/TreeSet",
        "isEmpty",
        "()Z",
        native_treeset_is_empty,
    );
    registry.natives_mut().register(
        "java/util/TreeSet",
        "iterator",
        "()Ljava/util/Iterator;",
        native_treeset_iterator,
    );

    // java/util/LinkedHashMap — insertion-order map (reuses HashMap layout)
    // Our HashMap already preserves insertion order via linear scan.
    // super_class=Object (not HashMap) to avoid double field count.
    let linked_hashmap_ctx = ClassContext {
        class_name: "java/util/LinkedHashMap".to_string(),
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
    registry.register(linked_hashmap_ctx);
    registry.natives_mut().register(
        "java/util/LinkedHashMap",
        "<init>",
        "()V",
        native_hashmap_init,
    );
    registry.natives_mut().register(
        "java/util/LinkedHashMap",
        "put",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        native_hashmap_put,
    );
    registry.natives_mut().register(
        "java/util/LinkedHashMap",
        "get",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_hashmap_get,
    );
    registry.natives_mut().register(
        "java/util/LinkedHashMap",
        "containsKey",
        "(Ljava/lang/Object;)Z",
        native_hashmap_contains_key,
    );
    registry.natives_mut().register(
        "java/util/LinkedHashMap",
        "size",
        "()I",
        native_hashmap_size,
    );
    registry.natives_mut().register(
        "java/util/LinkedHashMap",
        "remove",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_hashmap_remove,
    );
    registry.natives_mut().register(
        "java/util/LinkedHashMap",
        "isEmpty",
        "()Z",
        native_hashmap_is_empty,
    );
    registry.natives_mut().register(
        "java/util/LinkedHashMap",
        "getOrDefault",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        native_hashmap_get_or_default,
    );
    registry.natives_mut().register(
        "java/util/LinkedHashMap",
        "keySet",
        "()Ljava/util/Set;",
        native_hashmap_key_set,
    );
    registry.natives_mut().register(
        "java/util/LinkedHashMap",
        "values",
        "()Ljava/util/Collection;",
        native_hashmap_values,
    );
    registry.natives_mut().register(
        "java/util/LinkedHashMap",
        "entrySet",
        "()Ljava/util/Set;",
        native_hashmap_entry_set,
    );

    // duke/util/Stream — lazy pipeline backed by flat element array
    // fields[0] = Int(size), fields[1..] = element refs
    let stream_ctx = ClassContext {
        class_name: "duke/util/Stream".to_string(),
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
    registry.register(stream_ctx);

    // java/util/stream/Stream — public API alias for duke/util/Stream
    let jstream_ctx = ClassContext {
        class_name: "java/util/stream/Stream".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(jstream_ctx);
    registry.natives_mut().register(
        "java/util/stream/Stream",
        "of",
        "([Ljava/lang/Object;)Ljava/util/stream/Stream;",
        native_stream_of,
    );
    registry
        .natives_mut()
        .register("duke/util/Stream", "count", "()J", native_stream_count);
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "filter",
        "(Ljava/util/function/Predicate;)Ljava/util/stream/Stream;",
        native_stream_filter,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "map",
        "(Ljava/util/function/Function;)Ljava/util/stream/Stream;",
        native_stream_map,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "forEach",
        "(Ljava/util/function/Consumer;)V",
        native_stream_for_each,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "collect",
        "(Ljava/util/stream/Collector;)Ljava/lang/Object;",
        native_stream_collect,
    );
    registry.natives_mut().register(
        "duke/util/Stream",
        "distinct",
        "()Ljava/util/stream/Stream;",
        native_stream_distinct,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "sorted",
        "()Ljava/util/stream/Stream;",
        native_stream_sorted,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "anyMatch",
        "(Ljava/util/function/Predicate;)Z",
        native_stream_any_match,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "allMatch",
        "(Ljava/util/function/Predicate;)Z",
        native_stream_all_match,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "noneMatch",
        "(Ljava/util/function/Predicate;)Z",
        native_stream_none_match,
    );
    registry.natives_mut().register(
        "duke/util/Stream",
        "findFirst",
        "()Ljava/util/Optional;",
        native_stream_find_first,
    );
    registry.natives_mut().register(
        "duke/util/Stream",
        "findAny",
        "()Ljava/util/Optional;",
        native_stream_find_first,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "reduce",
        "(Ljava/util/function/BinaryOperator;)Ljava/util/Optional;",
        native_stream_reduce,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "reduce",
        "(Ljava/lang/Object;Ljava/util/function/BinaryOperator;)Ljava/lang/Object;",
        native_stream_reduce_with_identity,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "toList",
        "()Ljava/util/List;",
        native_stream_to_list,
    );
    // ArrayList.stream() — wraps ArrayList elements into a Stream
    registry.natives_mut().register(
        "java/util/ArrayList",
        "stream",
        "()Ljava/util/stream/Stream;",
        native_arraylist_stream,
    );

    // java/util/stream/Collectors — static factory for collectors
    let collectors_ctx = ClassContext {
        class_name: "java/util/stream/Collectors".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(collectors_ctx);
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "toList",
        "()Ljava/util/stream/Collector;",
        native_collectors_to_list,
    );

    // duke/util/ToListCollector — sentinel object for collect(Collectors.toList())
    let to_list_ctx = ClassContext {
        class_name: "duke/util/ToListCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(to_list_ctx);

    // duke/util/JoiningCollector — joining collector; fields[0]=delimiter, [1]=prefix, [2]=suffix
    let joining_ctx = ClassContext {
        class_name: "duke/util/JoiningCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "delimiter".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "prefix".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "suffix".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 3,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(joining_ctx);
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "joining",
        "(Ljava/lang/CharSequence;)Ljava/util/stream/Collector;",
        native_collectors_joining,
    );
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "joining",
        "()Ljava/util/stream/Collector;",
        native_collectors_joining_no_arg,
    );
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "joining",
        "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Ljava/util/stream/Collector;",
        native_collectors_joining_full,
    );

    // Stream.generate / iterate / concat / empty
    registry.natives_mut().register(
        "java/util/stream/Stream",
        "generate",
        "(Ljava/util/function/Supplier;)Ljava/util/stream/Stream;",
        native_stream_generate,
    );
    registry.natives_mut().register(
        "java/util/stream/Stream",
        "iterate",
        "(Ljava/lang/Object;Ljava/util/function/UnaryOperator;)Ljava/util/stream/Stream;",
        native_stream_iterate,
    );
    registry.natives_mut().register(
        "java/util/stream/Stream",
        "concat",
        "(Ljava/util/stream/Stream;Ljava/util/stream/Stream;)Ljava/util/stream/Stream;",
        native_stream_concat,
    );
    registry.natives_mut().register(
        "java/util/stream/Stream",
        "empty",
        "()Ljava/util/stream/Stream;",
        native_stream_empty,
    );
    // Register sentinel classes for lazy generator/iterator streams.
    registry.register(ClassContext {
        class_name: "duke/util/GeneratorStream".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "supplier".to_string(),
            descriptor: "Ljava/util/function/Supplier;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/stream/Stream".to_string()],
        bootstrap_methods: Vec::new(),
    });
    registry.register(ClassContext {
        class_name: "duke/util/IteratorStream".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "seed".to_string(),
                descriptor: "Ljava/lang/Object;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "fn".to_string(),
                descriptor: "Ljava/util/function/UnaryOperator;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/stream/Stream".to_string()],
        bootstrap_methods: Vec::new(),
    });

    // Stream.limit / skip / flatMap
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "limit",
        "(J)Ljava/util/stream/Stream;",
        native_stream_limit,
    );
    // Also register limit on sentinel streams so they can be materialised.
    registry.natives_mut().register_callback(
        "duke/util/GeneratorStream",
        "limit",
        "(J)Ljava/util/stream/Stream;",
        native_stream_limit,
    );
    registry.natives_mut().register_callback(
        "duke/util/IteratorStream",
        "limit",
        "(J)Ljava/util/stream/Stream;",
        native_stream_limit,
    );
    registry.natives_mut().register(
        "duke/util/Stream",
        "skip",
        "(J)Ljava/util/stream/Stream;",
        native_stream_skip,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "flatMap",
        "(Ljava/util/function/Function;)Ljava/util/stream/Stream;",
        native_stream_flat_map,
    );

    // duke/util/IntStream — unboxed int stream
    // fields[0]=Int(size), fields[1..n]=Int(value) elements
    let int_stream_ctx = ClassContext {
        class_name: "duke/util/IntStream".to_string(),
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
    registry.register(int_stream_ctx);

    // java/util/stream/IntStream — public API alias
    let j_int_stream_ctx = ClassContext {
        class_name: "java/util/stream/IntStream".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(j_int_stream_ctx);

    // IntStream static factories
    registry.natives_mut().register(
        "java/util/stream/IntStream",
        "range",
        "(II)Ljava/util/stream/IntStream;",
        native_int_stream_range,
    );
    registry.natives_mut().register(
        "java/util/stream/IntStream",
        "rangeClosed",
        "(II)Ljava/util/stream/IntStream;",
        native_int_stream_range_closed,
    );
    registry.natives_mut().register(
        "java/util/stream/IntStream",
        "of",
        "([I)Ljava/util/stream/IntStream;",
        native_int_stream_of,
    );

    // IntStream terminal ops
    registry.natives_mut().register(
        "duke/util/IntStream",
        "count",
        "()J",
        native_int_stream_count,
    );
    registry
        .natives_mut()
        .register("duke/util/IntStream", "sum", "()I", native_int_stream_sum);
    registry.natives_mut().register(
        "duke/util/IntStream",
        "min",
        "()Ljava/util/OptionalInt;",
        native_int_stream_min,
    );
    registry.natives_mut().register(
        "duke/util/IntStream",
        "max",
        "()Ljava/util/OptionalInt;",
        native_int_stream_max,
    );
    registry.natives_mut().register(
        "duke/util/IntStream",
        "average",
        "()Ljava/util/OptionalDouble;",
        native_int_stream_average,
    );
    registry.natives_mut().register(
        "duke/util/IntStream",
        "toArray",
        "()[I",
        native_int_stream_to_array,
    );

    // IntStream intermediate ops
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "filter",
        "(Ljava/util/function/IntPredicate;)Ljava/util/stream/IntStream;",
        native_int_stream_filter,
    );
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "map",
        "(Ljava/util/function/IntUnaryOperator;)Ljava/util/stream/IntStream;",
        native_int_stream_map,
    );
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "forEach",
        "(Ljava/util/function/IntConsumer;)V",
        native_int_stream_for_each,
    );
    registry.natives_mut().register(
        "duke/util/IntStream",
        "boxed",
        "()Ljava/util/stream/Stream;",
        native_int_stream_boxed,
    );
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "mapToObj",
        "(Ljava/util/function/IntFunction;)Ljava/util/stream/Stream;",
        native_int_stream_map_to_obj,
    );
    registry.natives_mut().register(
        "duke/util/IntStream",
        "distinct",
        "()Ljava/util/stream/IntStream;",
        native_int_stream_distinct,
    );

    // duke/util/OptionalInt — OptionalInt: fields[0]=Int(value), fields[1]=Int(present 0/1)
    let opt_int_ctx = ClassContext {
        class_name: "duke/util/OptionalInt".to_string(),
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
                name: "present".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(opt_int_ctx);
    // Also register as java/util/OptionalInt for dispatch
    let j_opt_int_ctx = ClassContext {
        class_name: "java/util/OptionalInt".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(j_opt_int_ctx);
    registry.natives_mut().register(
        "duke/util/OptionalInt",
        "getAsInt",
        "()I",
        native_optional_int_get_as_int,
    );
    registry.natives_mut().register(
        "duke/util/OptionalInt",
        "isPresent",
        "()Z",
        native_optional_int_is_present,
    );

    // duke/util/OptionalDouble
    let opt_dbl_ctx = ClassContext {
        class_name: "duke/util/OptionalDouble".to_string(),
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
                name: "present".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(opt_dbl_ctx);
    let j_opt_dbl_ctx = ClassContext {
        class_name: "java/util/OptionalDouble".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(j_opt_dbl_ctx);
    registry.natives_mut().register(
        "duke/util/OptionalDouble",
        "getAsDouble",
        "()D",
        native_optional_double_get_as_double,
    );

    // java/util/PriorityQueue — min-heap; fields[0]=Int(size), fields[1..]=heap array
    let pq_ctx = ClassContext {
        class_name: "java/util/PriorityQueue".to_string(),
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
    registry.register(pq_ctx);
    registry.natives_mut().register(
        "java/util/PriorityQueue",
        "<init>",
        "()V",
        native_priorityqueue_init,
    );
    registry.natives_mut().register_callback(
        "java/util/PriorityQueue",
        "offer",
        "(Ljava/lang/Object;)Z",
        native_priorityqueue_offer,
    );
    registry.natives_mut().register_callback(
        "java/util/PriorityQueue",
        "add",
        "(Ljava/lang/Object;)Z",
        native_priorityqueue_add,
    );
    registry.natives_mut().register(
        "java/util/PriorityQueue",
        "peek",
        "()Ljava/lang/Object;",
        native_priorityqueue_peek,
    );
    registry.natives_mut().register_callback(
        "java/util/PriorityQueue",
        "poll",
        "()Ljava/lang/Object;",
        native_priorityqueue_poll,
    );
    registry.natives_mut().register(
        "java/util/PriorityQueue",
        "size",
        "()I",
        native_priorityqueue_size,
    );
    registry.natives_mut().register(
        "java/util/PriorityQueue",
        "isEmpty",
        "()Z",
        native_priorityqueue_is_empty,
    );

    // java/util/ArrayDeque — double-ended queue backed by flat element array
    // fields[0] = Int(size), fields[1..] = elements (front at index 1)
    let arraydeque_ctx = ClassContext {
        class_name: "java/util/ArrayDeque".to_string(),
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
            "java/util/Deque".to_string(),
            "java/lang/Iterable".to_string(),
        ],
        bootstrap_methods: Vec::new(),
    };
    registry.register(arraydeque_ctx);
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "<init>",
        "()V",
        native_arraydeque_init,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "push",
        "(Ljava/lang/Object;)V",
        native_arraydeque_push,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "pop",
        "()Ljava/lang/Object;",
        native_arraydeque_pop,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "offer",
        "(Ljava/lang/Object;)Z",
        native_arraydeque_offer,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "add",
        "(Ljava/lang/Object;)Z",
        native_arraydeque_add,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "poll",
        "()Ljava/lang/Object;",
        native_arraydeque_poll,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "peek",
        "()Ljava/lang/Object;",
        native_arraydeque_peek,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "size",
        "()I",
        native_arraydeque_size,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "isEmpty",
        "()Z",
        native_arraydeque_is_empty,
    );

    // java/util/Objects — null-safe utility methods
    let objects_ctx = ClassContext {
        class_name: "java/util/Objects".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(objects_ctx);
    registry.natives_mut().register(
        "java/util/Objects",
        "isNull",
        "(Ljava/lang/Object;)Z",
        native_objects_is_null,
    );
    registry.natives_mut().register(
        "java/util/Objects",
        "nonNull",
        "(Ljava/lang/Object;)Z",
        native_objects_non_null,
    );
    registry.natives_mut().register(
        "java/util/Objects",
        "requireNonNull",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_objects_require_non_null,
    );
    registry.natives_mut().register(
        "java/util/Objects",
        "requireNonNull",
        "(Ljava/lang/Object;Ljava/lang/String;)Ljava/lang/Object;",
        native_objects_require_non_null_msg,
    );
    registry.natives_mut().register(
        "java/util/Objects",
        "equals",
        "(Ljava/lang/Object;Ljava/lang/Object;)Z",
        native_objects_equals,
    );
    registry.natives_mut().register(
        "java/util/Objects",
        "toString",
        "(Ljava/lang/Object;)Ljava/lang/String;",
        native_objects_tostring,
    );
    registry.natives_mut().register(
        "java/util/Objects",
        "hashCode",
        "(Ljava/lang/Object;)I",
        native_objects_hashcode,
    );

    // java/util/List — static factory `of` methods (0–6 fixed-arity + varargs)
    let list_ctx = ClassContext {
        class_name: "java/util/List".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(list_ctx);
    for desc in &[
        "()Ljava/util/List;",
        "(Ljava/lang/Object;)Ljava/util/List;",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/List;",
        "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/List;",
        "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/List;",
        "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/List;",
        "([Ljava/lang/Object;)Ljava/util/List;",
    ] {
        registry
            .natives_mut()
            .register("java/util/List", "of", desc, native_list_of);
    }
    registry.natives_mut().register(
        "java/util/List",
        "copyOf",
        "(Ljava/util/Collection;)Ljava/util/List;",
        native_list_of,
    );

    // java/util/Set — static factory `of` methods
    let set_iface_ctx = ClassContext {
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
    registry.register(set_iface_ctx);
    for desc in &[
        "()Ljava/util/Set;",
        "(Ljava/lang/Object;)Ljava/util/Set;",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/Set;",
        "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/Set;",
        "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/Set;",
        "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/Set;",
        "([Ljava/lang/Object;)Ljava/util/Set;",
    ] {
        registry
            .natives_mut()
            .register("java/util/Set", "of", desc, native_set_of_factory);
    }
    registry.natives_mut().register(
        "java/util/Set",
        "copyOf",
        "(Ljava/util/Collection;)Ljava/util/Set;",
        native_set_of_factory,
    );

    // java/util/Map — static factory `of` methods (pairs of args)
    let map_iface_ctx = ClassContext {
        class_name: "java/util/Map".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(map_iface_ctx);
    for desc in &[
        "()Ljava/util/Map;",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/Map;",
        "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/Map;",
        "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/Map;",
    ] {
        registry
            .natives_mut()
            .register("java/util/Map", "of", desc, native_map_of);
    }

    // java/util/Optional — null-safe value container
    let optional_ctx = ClassContext {
        class_name: "java/util/Optional".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "value".to_string(),
            descriptor: "Ljava/lang/Object;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(optional_ctx);
    registry.natives_mut().register(
        "java/util/Optional",
        "empty",
        "()Ljava/util/Optional;",
        native_optional_empty,
    );
    registry.natives_mut().register(
        "java/util/Optional",
        "of",
        "(Ljava/lang/Object;)Ljava/util/Optional;",
        native_optional_of,
    );
    registry.natives_mut().register(
        "java/util/Optional",
        "ofNullable",
        "(Ljava/lang/Object;)Ljava/util/Optional;",
        native_optional_of_nullable,
    );
    registry.natives_mut().register(
        "java/util/Optional",
        "get",
        "()Ljava/lang/Object;",
        native_optional_get,
    );
    registry.natives_mut().register(
        "java/util/Optional",
        "isPresent",
        "()Z",
        native_optional_is_present,
    );
    registry.natives_mut().register(
        "java/util/Optional",
        "isEmpty",
        "()Z",
        native_optional_is_empty,
    );
    registry.natives_mut().register(
        "java/util/Optional",
        "orElse",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_optional_or_else,
    );
    registry.natives_mut().register(
        "java/util/Optional",
        "orElseThrow",
        "()Ljava/lang/Object;",
        native_optional_or_else_throw,
    );
    registry.natives_mut().register_callback(
        "java/util/Optional",
        "map",
        "(Ljava/util/function/Function;)Ljava/util/Optional;",
        native_optional_map,
    );
    registry.natives_mut().register_callback(
        "java/util/Optional",
        "flatMap",
        "(Ljava/util/function/Function;)Ljava/util/Optional;",
        native_optional_flat_map,
    );
    registry.natives_mut().register_callback(
        "java/util/Optional",
        "filter",
        "(Ljava/util/function/Predicate;)Ljava/util/Optional;",
        native_optional_filter,
    );
    registry.natives_mut().register_callback(
        "java/util/Optional",
        "ifPresent",
        "(Ljava/util/function/Consumer;)V",
        native_optional_if_present,
    );
    registry.natives_mut().register_callback(
        "java/util/Optional",
        "orElseGet",
        "(Ljava/util/function/Supplier;)Ljava/lang/Object;",
        native_optional_or_else_get,
    );

    // ArrayList.addAll and HashMap.putAll
    registry.natives_mut().register(
        "java/util/ArrayList",
        "addAll",
        "(Ljava/util/Collection;)Z",
        native_arraylist_add_all,
    );
    registry.natives_mut().register(
        "java/util/HashMap",
        "putAll",
        "(Ljava/util/Map;)V",
        native_hashmap_put_all,
    );
    registry.natives_mut().register_callback(
        "java/util/HashMap",
        "computeIfAbsent",
        "(Ljava/lang/Object;Ljava/util/function/Function;)Ljava/lang/Object;",
        native_hashmap_compute_if_absent,
    );
    registry.natives_mut().register_callback(
        "java/util/HashMap",
        "compute",
        "(Ljava/lang/Object;Ljava/util/function/BiFunction;)Ljava/lang/Object;",
        native_hashmap_compute,
    );
    registry.natives_mut().register_callback(
        "java/util/HashMap",
        "merge",
        "(Ljava/lang/Object;Ljava/lang/Object;Ljava/util/function/BiFunction;)Ljava/lang/Object;",
        native_hashmap_merge,
    );

    // java/util/regex/Pattern — compiled regex pattern, string_value = regex string
    let pattern_ctx = ClassContext {
        class_name: "java/util/regex/Pattern".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(pattern_ctx);
    registry.natives_mut().register(
        "java/util/regex/Pattern",
        "compile",
        "(Ljava/lang/String;)Ljava/util/regex/Pattern;",
        native_pattern_compile,
    );
    registry.natives_mut().register(
        "java/util/regex/Pattern",
        "matcher",
        "(Ljava/lang/CharSequence;)Ljava/util/regex/Matcher;",
        native_pattern_matcher,
    );
    registry.natives_mut().register(
        "java/util/regex/Pattern",
        "matches",
        "(Ljava/lang/String;Ljava/lang/CharSequence;)Z",
        native_pattern_matches_static,
    );

    // java/util/regex/Matcher — stateful matcher
    // fields[0]=Pattern, [1]=input, [2]=pos, [3]=match_start, [4]=match_end
    let matcher_ctx = ClassContext {
        class_name: "java/util/regex/Matcher".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "pattern".to_string(),
                descriptor: "Ljava/util/regex/Pattern;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "input".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "pos".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "matchStart".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "matchEnd".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 5,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(matcher_ctx);
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "find",
        "()Z",
        native_matcher_find,
    );
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "matches",
        "()Z",
        native_matcher_matches,
    );
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "group",
        "()Ljava/lang/String;",
        native_matcher_group,
    );
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "start",
        "()I",
        native_matcher_start,
    );
    registry
        .natives_mut()
        .register("java/util/regex/Matcher", "end", "()I", native_matcher_end);
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "replaceAll",
        "(Ljava/lang/String;)Ljava/lang/String;",
        native_matcher_replace_all,
    );
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "replaceFirst",
        "(Ljava/lang/String;)Ljava/lang/String;",
        native_matcher_replace_first,
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

    // java/util/Random — 48-bit LCG pseudo-random number generator
    // fields[0] = Long(seed)
    let random_ctx = ClassContext {
        class_name: "java/util/Random".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "seed".to_string(),
            descriptor: "J".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(random_ctx);
    registry
        .natives_mut()
        .register("java/util/Random", "<init>", "()V", native_random_init);
    registry.natives_mut().register(
        "java/util/Random",
        "<init>",
        "(J)V",
        native_random_init_seed,
    );
    registry
        .natives_mut()
        .register("java/util/Random", "nextInt", "()I", native_random_next_int);
    registry.natives_mut().register(
        "java/util/Random",
        "nextInt",
        "(I)I",
        native_random_next_int_bound,
    );
    registry.natives_mut().register(
        "java/util/Random",
        "nextLong",
        "()J",
        native_random_next_long,
    );
    registry.natives_mut().register(
        "java/util/Random",
        "nextDouble",
        "()D",
        native_random_next_double,
    );
    registry.natives_mut().register(
        "java/util/Random",
        "nextFloat",
        "()F",
        native_random_next_float,
    );
    registry.natives_mut().register(
        "java/util/Random",
        "nextBoolean",
        "()Z",
        native_random_next_boolean,
    );

    // java/lang/StringBuffer — mutable string (thread-safe in Java; here aliases StringBuilder)
    // string_value used as the buffer
    let sb_buf_ctx = ClassContext {
        class_name: "java/lang/StringBuffer".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/lang/CharSequence".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(sb_buf_ctx);
    registry.natives_mut().register(
        "java/lang/StringBuffer",
        "<init>",
        "()V",
        native_stringbuffer_init,
    );
    registry.natives_mut().register(
        "java/lang/StringBuffer",
        "<init>",
        "(Ljava/lang/String;)V",
        native_stringbuffer_init_string,
    );
    registry.natives_mut().register(
        "java/lang/StringBuffer",
        "append",
        "(Ljava/lang/String;)Ljava/lang/StringBuffer;",
        native_stringbuffer_append,
    );
    registry.natives_mut().register(
        "java/lang/StringBuffer",
        "append",
        "(I)Ljava/lang/StringBuffer;",
        native_stringbuffer_append,
    );
    registry.natives_mut().register(
        "java/lang/StringBuffer",
        "append",
        "(J)Ljava/lang/StringBuffer;",
        native_stringbuffer_append,
    );
    registry.natives_mut().register(
        "java/lang/StringBuffer",
        "append",
        "(D)Ljava/lang/StringBuffer;",
        native_stringbuffer_append,
    );
    registry.natives_mut().register(
        "java/lang/StringBuffer",
        "append",
        "(Z)Ljava/lang/StringBuffer;",
        native_stringbuffer_append,
    );
    registry.natives_mut().register(
        "java/lang/StringBuffer",
        "append",
        "(Ljava/lang/Object;)Ljava/lang/StringBuffer;",
        native_stringbuffer_append,
    );
    registry.natives_mut().register(
        "java/lang/StringBuffer",
        "toString",
        "()Ljava/lang/String;",
        native_stringbuffer_tostring,
    );
    registry.natives_mut().register(
        "java/lang/StringBuffer",
        "length",
        "()I",
        native_stringbuffer_length,
    );

    // java/util/StringJoiner — joins strings with delimiter, optional prefix/suffix
    // fields[0]=delimiter, fields[1]=prefix, fields[2]=suffix, fields[3]=emptyValue,
    // fields[4..]=added elements
    let sj_ctx = ClassContext {
        class_name: "java/util/StringJoiner".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "delimiter".to_string(),
                descriptor: "Ljava/lang/CharSequence;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "prefix".to_string(),
                descriptor: "Ljava/lang/CharSequence;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "suffix".to_string(),
                descriptor: "Ljava/lang/CharSequence;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "emptyValue".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 4,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(sj_ctx);
    registry.natives_mut().register(
        "java/util/StringJoiner",
        "<init>",
        "(Ljava/lang/CharSequence;)V",
        native_stringjoiner_init,
    );
    registry.natives_mut().register(
        "java/util/StringJoiner",
        "<init>",
        "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;Ljava/lang/CharSequence;)V",
        native_stringjoiner_init_prefix_suffix,
    );
    registry.natives_mut().register(
        "java/util/StringJoiner",
        "add",
        "(Ljava/lang/CharSequence;)Ljava/util/StringJoiner;",
        native_stringjoiner_add,
    );
    registry.natives_mut().register(
        "java/util/StringJoiner",
        "setEmptyValue",
        "(Ljava/lang/CharSequence;)Ljava/util/StringJoiner;",
        native_stringjoiner_set_empty_value,
    );
    registry.natives_mut().register(
        "java/util/StringJoiner",
        "toString",
        "()Ljava/lang/String;",
        native_stringjoiner_tostring,
    );
    registry.natives_mut().register(
        "java/util/StringJoiner",
        "length",
        "()I",
        native_stringjoiner_length,
    );

    // -----------------------------------------------------------------------
    // Phase 49: Comparator.thenComparing, Predicate combinators,
    //           Function combinators, Stream.mapToLong/mapToDouble
    // -----------------------------------------------------------------------

    // Comparator.thenComparing(Comparator) → ThenComparingComparator
    registry.natives_mut().register(
        "duke/util/ComparingIntComparator",
        "thenComparing",
        "(Ljava/util/Comparator;)Ljava/util/Comparator;",
        native_comparator_then_comparing,
    );
    registry.natives_mut().register(
        "duke/util/ComparingComparator",
        "thenComparing",
        "(Ljava/util/Comparator;)Ljava/util/Comparator;",
        native_comparator_then_comparing,
    );
    registry.natives_mut().register(
        "duke/util/ReversedComparator",
        "thenComparing",
        "(Ljava/util/Comparator;)Ljava/util/Comparator;",
        native_comparator_then_comparing,
    );
    let then_cmp_ctx = ClassContext {
        class_name: "duke/util/ThenComparingComparator".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "primary".to_string(),
                descriptor: "Ljava/util/Comparator;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "secondary".to_string(),
                descriptor: "Ljava/util/Comparator;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/Comparator".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(then_cmp_ctx);
    registry.natives_mut().register_callback(
        "duke/util/ThenComparingComparator",
        "compare",
        "(Ljava/lang/Object;Ljava/lang/Object;)I",
        native_then_comparing_compare,
    );
    registry.natives_mut().register(
        "duke/util/ThenComparingComparator",
        "thenComparing",
        "(Ljava/util/Comparator;)Ljava/util/Comparator;",
        native_comparator_then_comparing,
    );

    // Predicate.and / or / negate combinators
    // Register on lambda class placeholder — actual dispatch via runtime class
    registry.natives_mut().register(
        "java/util/function/Predicate",
        "and",
        "(Ljava/util/function/Predicate;)Ljava/util/function/Predicate;",
        native_predicate_and,
    );
    registry.natives_mut().register(
        "java/util/function/Predicate",
        "or",
        "(Ljava/util/function/Predicate;)Ljava/util/function/Predicate;",
        native_predicate_or,
    );
    registry.natives_mut().register(
        "java/util/function/Predicate",
        "negate",
        "()Ljava/util/function/Predicate;",
        native_predicate_negate,
    );
    let and_pred_ctx = ClassContext {
        class_name: "duke/util/AndPredicate".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "left".to_string(),
                descriptor: "Ljava/util/function/Predicate;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "right".to_string(),
                descriptor: "Ljava/util/function/Predicate;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/function/Predicate".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(and_pred_ctx);
    registry.natives_mut().register_callback(
        "duke/util/AndPredicate",
        "test",
        "(Ljava/lang/Object;)Z",
        native_and_predicate_test,
    );
    registry.natives_mut().register(
        "duke/util/AndPredicate",
        "and",
        "(Ljava/util/function/Predicate;)Ljava/util/function/Predicate;",
        native_predicate_and,
    );
    registry.natives_mut().register(
        "duke/util/AndPredicate",
        "or",
        "(Ljava/util/function/Predicate;)Ljava/util/function/Predicate;",
        native_predicate_or,
    );
    registry.natives_mut().register(
        "duke/util/AndPredicate",
        "negate",
        "()Ljava/util/function/Predicate;",
        native_predicate_negate,
    );
    let or_pred_ctx = ClassContext {
        class_name: "duke/util/OrPredicate".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "left".to_string(),
                descriptor: "Ljava/util/function/Predicate;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "right".to_string(),
                descriptor: "Ljava/util/function/Predicate;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/function/Predicate".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(or_pred_ctx);
    registry.natives_mut().register_callback(
        "duke/util/OrPredicate",
        "test",
        "(Ljava/lang/Object;)Z",
        native_or_predicate_test,
    );
    registry.natives_mut().register(
        "duke/util/OrPredicate",
        "and",
        "(Ljava/util/function/Predicate;)Ljava/util/function/Predicate;",
        native_predicate_and,
    );
    registry.natives_mut().register(
        "duke/util/OrPredicate",
        "or",
        "(Ljava/util/function/Predicate;)Ljava/util/function/Predicate;",
        native_predicate_or,
    );
    registry.natives_mut().register(
        "duke/util/OrPredicate",
        "negate",
        "()Ljava/util/function/Predicate;",
        native_predicate_negate,
    );
    let neg_pred_ctx = ClassContext {
        class_name: "duke/util/NegatedPredicate".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "original".to_string(),
            descriptor: "Ljava/util/function/Predicate;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/function/Predicate".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(neg_pred_ctx);
    registry.natives_mut().register_callback(
        "duke/util/NegatedPredicate",
        "test",
        "(Ljava/lang/Object;)Z",
        native_negated_predicate_test,
    );
    registry.natives_mut().register(
        "duke/util/NegatedPredicate",
        "and",
        "(Ljava/util/function/Predicate;)Ljava/util/function/Predicate;",
        native_predicate_and,
    );
    registry.natives_mut().register(
        "duke/util/NegatedPredicate",
        "or",
        "(Ljava/util/function/Predicate;)Ljava/util/function/Predicate;",
        native_predicate_or,
    );
    registry.natives_mut().register(
        "duke/util/NegatedPredicate",
        "negate",
        "()Ljava/util/function/Predicate;",
        native_predicate_negate,
    );

    // Function.andThen / compose combinators
    registry.natives_mut().register(
        "java/util/function/Function",
        "andThen",
        "(Ljava/util/function/Function;)Ljava/util/function/Function;",
        native_function_and_then,
    );
    registry.natives_mut().register(
        "java/util/function/Function",
        "compose",
        "(Ljava/util/function/Function;)Ljava/util/function/Function;",
        native_function_compose,
    );
    let and_then_fn_ctx = ClassContext {
        class_name: "duke/util/AndThenFunction".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "first".to_string(),
                descriptor: "Ljava/util/function/Function;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "second".to_string(),
                descriptor: "Ljava/util/function/Function;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/function/Function".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(and_then_fn_ctx);
    registry.natives_mut().register_callback(
        "duke/util/AndThenFunction",
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_and_then_function_apply,
    );
    registry.natives_mut().register(
        "duke/util/AndThenFunction",
        "andThen",
        "(Ljava/util/function/Function;)Ljava/util/function/Function;",
        native_function_and_then,
    );
    registry.natives_mut().register(
        "duke/util/AndThenFunction",
        "compose",
        "(Ljava/util/function/Function;)Ljava/util/function/Function;",
        native_function_compose,
    );
    let compose_fn_ctx = ClassContext {
        class_name: "duke/util/ComposeFunction".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "outer".to_string(),
                descriptor: "Ljava/util/function/Function;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "inner".to_string(),
                descriptor: "Ljava/util/function/Function;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/function/Function".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(compose_fn_ctx);
    registry.natives_mut().register_callback(
        "duke/util/ComposeFunction",
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_compose_function_apply,
    );
    registry.natives_mut().register(
        "duke/util/ComposeFunction",
        "andThen",
        "(Ljava/util/function/Function;)Ljava/util/function/Function;",
        native_function_and_then,
    );
    registry.natives_mut().register(
        "duke/util/ComposeFunction",
        "compose",
        "(Ljava/util/function/Function;)Ljava/util/function/Function;",
        native_function_compose,
    );

    // Stream.mapToLong → LongStream
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "mapToLong",
        "(Ljava/util/function/ToLongFunction;)Ljava/util/stream/LongStream;",
        native_stream_map_to_long,
    );
    let long_stream_ctx = ClassContext {
        class_name: "duke/util/LongStream".to_string(),
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
        interfaces: vec!["java/util/stream/LongStream".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(long_stream_ctx);
    registry
        .natives_mut()
        .register("duke/util/LongStream", "sum", "()J", native_long_stream_sum);

    // Stream.mapToDouble → DoubleStream
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "mapToDouble",
        "(Ljava/util/function/ToDoubleFunction;)Ljava/util/stream/DoubleStream;",
        native_stream_map_to_double,
    );
    let double_stream_ctx = ClassContext {
        class_name: "duke/util/DoubleStream".to_string(),
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
        interfaces: vec!["java/util/stream/DoubleStream".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(double_stream_ctx);
    registry.natives_mut().register(
        "duke/util/DoubleStream",
        "sum",
        "()D",
        native_double_stream_sum,
    );

    // -----------------------------------------------------------------------
    // Phase 51: Full LongStream, DoubleStream ops, IntStream.asLongStream/asDoubleStream,
    //           Collectors.summingInt/averagingInt
    // -----------------------------------------------------------------------

    // LongStream static factories
    registry.natives_mut().register(
        "java/util/stream/LongStream",
        "of",
        "([J)Ljava/util/stream/LongStream;",
        native_long_stream_of,
    );
    registry.natives_mut().register(
        "java/util/stream/LongStream",
        "range",
        "(JJ)Ljava/util/stream/LongStream;",
        native_long_stream_range,
    );
    registry.natives_mut().register(
        "java/util/stream/LongStream",
        "rangeClosed",
        "(JJ)Ljava/util/stream/LongStream;",
        native_long_stream_range_closed,
    );

    // LongStream terminal ops
    registry.natives_mut().register(
        "duke/util/LongStream",
        "count",
        "()J",
        native_long_stream_count,
    );
    registry.natives_mut().register(
        "duke/util/LongStream",
        "min",
        "()Ljava/util/OptionalLong;",
        native_long_stream_min,
    );
    registry.natives_mut().register(
        "duke/util/LongStream",
        "max",
        "()Ljava/util/OptionalLong;",
        native_long_stream_max,
    );
    registry.natives_mut().register(
        "duke/util/LongStream",
        "average",
        "()Ljava/util/OptionalDouble;",
        native_long_stream_average,
    );
    registry.natives_mut().register(
        "duke/util/LongStream",
        "toArray",
        "()[J",
        native_long_stream_to_array,
    );
    registry.natives_mut().register(
        "duke/util/LongStream",
        "sorted",
        "()Ljava/util/stream/LongStream;",
        native_long_stream_sorted,
    );
    registry.natives_mut().register(
        "duke/util/LongStream",
        "distinct",
        "()Ljava/util/stream/LongStream;",
        native_long_stream_distinct,
    );
    registry.natives_mut().register_callback(
        "duke/util/LongStream",
        "reduce",
        "(JLjava/util/function/LongBinaryOperator;)J",
        native_long_stream_reduce_identity,
    );
    registry.natives_mut().register(
        "duke/util/LongStream",
        "boxed",
        "()Ljava/util/stream/Stream;",
        native_long_stream_boxed,
    );

    // LongStream intermediate ops
    registry.natives_mut().register_callback(
        "duke/util/LongStream",
        "filter",
        "(Ljava/util/function/LongPredicate;)Ljava/util/stream/LongStream;",
        native_long_stream_filter,
    );
    registry.natives_mut().register_callback(
        "duke/util/LongStream",
        "map",
        "(Ljava/util/function/LongUnaryOperator;)Ljava/util/stream/LongStream;",
        native_long_stream_map,
    );
    registry.natives_mut().register_callback(
        "duke/util/LongStream",
        "forEach",
        "(Ljava/util/function/LongConsumer;)V",
        native_long_stream_for_each,
    );
    registry.natives_mut().register_callback(
        "duke/util/LongStream",
        "mapToInt",
        "(Ljava/util/function/LongToIntFunction;)Ljava/util/stream/IntStream;",
        native_long_stream_map_to_int,
    );

    // OptionalLong synthetic class
    let opt_long_ctx = ClassContext {
        class_name: "duke/util/OptionalLong".to_string(),
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
                name: "present".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(opt_long_ctx);
    // Also register as java/util/OptionalLong for dispatch
    let opt_long_ctx2 = ClassContext {
        class_name: "java/util/OptionalLong".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(opt_long_ctx2);
    registry.natives_mut().register(
        "duke/util/OptionalLong",
        "getAsLong",
        "()J",
        native_optional_long_get_as_long,
    );
    registry.natives_mut().register(
        "duke/util/OptionalLong",
        "isPresent",
        "()Z",
        native_optional_long_is_present,
    );

    // DoubleStream static factory (varargs and single-element forms)
    registry.natives_mut().register(
        "java/util/stream/DoubleStream",
        "of",
        "([D)Ljava/util/stream/DoubleStream;",
        native_double_stream_of,
    );
    registry.natives_mut().register(
        "java/util/stream/DoubleStream",
        "of",
        "(D)Ljava/util/stream/DoubleStream;",
        native_double_stream_of_single,
    );

    // DoubleStream terminal ops (in addition to sum already registered)
    registry.natives_mut().register(
        "duke/util/DoubleStream",
        "count",
        "()J",
        native_double_stream_count,
    );
    registry.natives_mut().register(
        "duke/util/DoubleStream",
        "min",
        "()Ljava/util/OptionalDouble;",
        native_double_stream_min,
    );
    registry.natives_mut().register(
        "duke/util/DoubleStream",
        "max",
        "()Ljava/util/OptionalDouble;",
        native_double_stream_max,
    );
    registry.natives_mut().register(
        "duke/util/DoubleStream",
        "average",
        "()Ljava/util/OptionalDouble;",
        native_double_stream_average,
    );
    registry.natives_mut().register(
        "duke/util/DoubleStream",
        "toArray",
        "()[D",
        native_double_stream_to_array,
    );
    registry.natives_mut().register(
        "duke/util/DoubleStream",
        "sorted",
        "()Ljava/util/stream/DoubleStream;",
        native_double_stream_sorted,
    );

    // DoubleStream intermediate ops
    registry.natives_mut().register_callback(
        "duke/util/DoubleStream",
        "filter",
        "(Ljava/util/function/DoublePredicate;)Ljava/util/stream/DoubleStream;",
        native_double_stream_filter,
    );
    registry.natives_mut().register_callback(
        "duke/util/DoubleStream",
        "map",
        "(Ljava/util/function/DoubleUnaryOperator;)Ljava/util/stream/DoubleStream;",
        native_double_stream_map,
    );

    // IntStream.asLongStream / asDoubleStream
    registry.natives_mut().register(
        "duke/util/IntStream",
        "asLongStream",
        "()Ljava/util/stream/LongStream;",
        native_int_stream_as_long_stream,
    );
    registry.natives_mut().register(
        "duke/util/IntStream",
        "asDoubleStream",
        "()Ljava/util/stream/DoubleStream;",
        native_int_stream_as_double_stream,
    );

    // Collectors.summingInt / averagingInt
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "summingInt",
        "(Ljava/util/function/ToIntFunction;)Ljava/util/stream/Collector;",
        native_collectors_summing_int,
    );
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "averagingInt",
        "(Ljava/util/function/ToIntFunction;)Ljava/util/stream/Collector;",
        native_collectors_averaging_int,
    );
    // SummingIntCollector and AveragingIntCollector sentinel classes
    let summing_ctx = ClassContext {
        class_name: "duke/util/SummingIntCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fn".to_string(),
            descriptor: "Ljava/util/function/ToIntFunction;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(summing_ctx);
    let averaging_ctx = ClassContext {
        class_name: "duke/util/AveragingIntCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fn".to_string(),
            descriptor: "Ljava/util/function/ToIntFunction;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(averaging_ctx);

    // ---------------------------------------------------------------------------
    // Phase 53: IntStream/LongStream terminal ops, Comparator.comparingLong,
    //           Optional.or / ifPresentOrElse, Collectors.toUnmodifiable*
    // ---------------------------------------------------------------------------

    // IntStream.findFirst / anyMatch / allMatch / noneMatch
    registry.natives_mut().register(
        "duke/util/IntStream",
        "findFirst",
        "()Ljava/util/OptionalInt;",
        native_int_stream_find_first,
    );
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "anyMatch",
        "(Ljava/util/function/IntPredicate;)Z",
        native_int_stream_any_match,
    );
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "allMatch",
        "(Ljava/util/function/IntPredicate;)Z",
        native_int_stream_all_match,
    );
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "noneMatch",
        "(Ljava/util/function/IntPredicate;)Z",
        native_int_stream_none_match,
    );

    // IntStream.mapToLong(IntToLongFunction) → LongStream
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "mapToLong",
        "(Ljava/util/function/IntToLongFunction;)Ljava/util/stream/LongStream;",
        native_int_stream_map_to_long,
    );

    // LongStream.findFirst / anyMatch / allMatch / noneMatch
    registry.natives_mut().register(
        "duke/util/LongStream",
        "findFirst",
        "()Ljava/util/OptionalLong;",
        native_long_stream_find_first,
    );
    registry.natives_mut().register_callback(
        "duke/util/LongStream",
        "anyMatch",
        "(Ljava/util/function/LongPredicate;)Z",
        native_long_stream_any_match,
    );
    registry.natives_mut().register_callback(
        "duke/util/LongStream",
        "allMatch",
        "(Ljava/util/function/LongPredicate;)Z",
        native_long_stream_all_match,
    );
    registry.natives_mut().register_callback(
        "duke/util/LongStream",
        "noneMatch",
        "(Ljava/util/function/LongPredicate;)Z",
        native_long_stream_none_match,
    );

    // Comparator.comparingLong(ToLongFunction)
    registry.natives_mut().register(
        "java/util/Comparator",
        "comparingLong",
        "(Ljava/util/function/ToLongFunction;)Ljava/util/Comparator;",
        native_comparator_comparing_long,
    );
    let comparing_long_ctx = ClassContext {
        class_name: "duke/util/ComparingLongComparator".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "keyExtractor".to_string(),
            descriptor: "Ljava/util/function/ToLongFunction;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/Comparator".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(comparing_long_ctx);
    registry.natives_mut().register_callback(
        "duke/util/ComparingLongComparator",
        "compare",
        "(Ljava/lang/Object;Ljava/lang/Object;)I",
        native_comparing_long_compare,
    );

    // Optional.or(Supplier<Optional>) and Optional.ifPresentOrElse(Consumer, Runnable)
    registry.natives_mut().register_callback(
        "java/util/Optional",
        "or",
        "(Ljava/util/function/Supplier;)Ljava/util/Optional;",
        native_optional_or,
    );
    registry.natives_mut().register_callback(
        "java/util/Optional",
        "ifPresentOrElse",
        "(Ljava/util/function/Consumer;Ljava/lang/Runnable;)V",
        native_optional_if_present_or_else,
    );

    // Collectors.toUnmodifiableList() and toUnmodifiableSet() (Java 10)
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "toUnmodifiableList",
        "()Ljava/util/stream/Collector;",
        native_collectors_to_unmodifiable_list,
    );
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "toUnmodifiableSet",
        "()Ljava/util/stream/Collector;",
        native_collectors_to_unmodifiable_set,
    );

    // ---------------------------------------------------------------------------
    // Phase 54: IntStream/LongStream/DoubleStream limit/skip,
    //           IntStream/LongStream flatMap, Collectors.mapping + groupingBy 2-arg
    // ---------------------------------------------------------------------------

    // IntStream.limit / skip
    registry.natives_mut().register(
        "duke/util/IntStream",
        "limit",
        "(J)Ljava/util/stream/IntStream;",
        native_int_stream_limit,
    );
    registry.natives_mut().register(
        "duke/util/IntStream",
        "skip",
        "(J)Ljava/util/stream/IntStream;",
        native_int_stream_skip,
    );

    // IntStream.flatMap(IntFunction<IntStream>)IntStream
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "flatMap",
        "(Ljava/util/function/IntFunction;)Ljava/util/stream/IntStream;",
        native_int_stream_flat_map,
    );

    // LongStream.limit / skip
    registry.natives_mut().register(
        "duke/util/LongStream",
        "limit",
        "(J)Ljava/util/stream/LongStream;",
        native_long_stream_limit,
    );
    registry.natives_mut().register(
        "duke/util/LongStream",
        "skip",
        "(J)Ljava/util/stream/LongStream;",
        native_long_stream_skip,
    );

    // LongStream.flatMap(LongFunction<LongStream>)LongStream
    registry.natives_mut().register_callback(
        "duke/util/LongStream",
        "flatMap",
        "(Ljava/util/function/LongFunction;)Ljava/util/stream/LongStream;",
        native_long_stream_flat_map,
    );

    // DoubleStream.limit / skip
    registry.natives_mut().register(
        "duke/util/DoubleStream",
        "limit",
        "(J)Ljava/util/stream/DoubleStream;",
        native_double_stream_limit,
    );
    registry.natives_mut().register(
        "duke/util/DoubleStream",
        "skip",
        "(J)Ljava/util/stream/DoubleStream;",
        native_double_stream_skip,
    );

    // Collectors.mapping(Function, Collector) — MappingCollector sentinel
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "mapping",
        "(Ljava/util/function/Function;Ljava/util/stream/Collector;)Ljava/util/stream/Collector;",
        native_collectors_mapping,
    );
    let mapping_ctx = ClassContext {
        class_name: "duke/util/MappingCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "mapper".to_string(),
                descriptor: "Ljava/util/function/Function;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "downstream".to_string(),
                descriptor: "Ljava/util/stream/Collector;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(mapping_ctx);

    // Collectors.groupingBy(Function, Collector) — 2-arg version with downstream
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "groupingBy",
        "(Ljava/util/function/Function;Ljava/util/stream/Collector;)Ljava/util/stream/Collector;",
        native_collectors_grouping_by_2,
    );
    let grouping2_ctx = ClassContext {
        class_name: "duke/util/GroupingBy2Collector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "keyFn".to_string(),
                descriptor: "Ljava/util/function/Function;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "downstream".to_string(),
                descriptor: "Ljava/util/stream/Collector;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    };
    registry.register(grouping2_ctx);

    // ---------------------------------------------------------------------------
    // Phase 55: Complete DoubleStream, LongStream gaps, Collectors.summingLong/averagingDouble
    // ---------------------------------------------------------------------------

    // DoubleStream — forEach, anyMatch/allMatch/noneMatch, findFirst
    registry.natives_mut().register_callback(
        "duke/util/DoubleStream",
        "forEach",
        "(Ljava/util/function/DoubleConsumer;)V",
        native_double_stream_for_each,
    );
    registry.natives_mut().register_callback(
        "duke/util/DoubleStream",
        "anyMatch",
        "(Ljava/util/function/DoublePredicate;)Z",
        native_double_stream_any_match,
    );
    registry.natives_mut().register_callback(
        "duke/util/DoubleStream",
        "allMatch",
        "(Ljava/util/function/DoublePredicate;)Z",
        native_double_stream_all_match,
    );
    registry.natives_mut().register_callback(
        "duke/util/DoubleStream",
        "noneMatch",
        "(Ljava/util/function/DoublePredicate;)Z",
        native_double_stream_none_match,
    );
    registry.natives_mut().register(
        "duke/util/DoubleStream",
        "findFirst",
        "()Ljava/util/OptionalDouble;",
        native_double_stream_find_first,
    );

    // DoubleStream — reduce (identity + optional)
    registry.natives_mut().register_callback(
        "duke/util/DoubleStream",
        "reduce",
        "(DLjava/util/function/DoubleBinaryOperator;)D",
        native_double_stream_reduce_identity,
    );
    registry.natives_mut().register_callback(
        "duke/util/DoubleStream",
        "reduce",
        "(Ljava/util/function/DoubleBinaryOperator;)Ljava/util/OptionalDouble;",
        native_double_stream_reduce_optional,
    );

    // DoubleStream — flatMap, mapToInt, mapToLong, distinct, boxed
    registry.natives_mut().register_callback(
        "duke/util/DoubleStream",
        "flatMap",
        "(Ljava/util/function/DoubleFunction;)Ljava/util/stream/DoubleStream;",
        native_double_stream_flat_map,
    );
    registry.natives_mut().register_callback(
        "duke/util/DoubleStream",
        "mapToInt",
        "(Ljava/util/function/DoubleToIntFunction;)Ljava/util/stream/IntStream;",
        native_double_stream_map_to_int,
    );
    registry.natives_mut().register_callback(
        "duke/util/DoubleStream",
        "mapToLong",
        "(Ljava/util/function/DoubleToLongFunction;)Ljava/util/stream/LongStream;",
        native_double_stream_map_to_long,
    );
    registry.natives_mut().register(
        "duke/util/DoubleStream",
        "distinct",
        "()Ljava/util/stream/DoubleStream;",
        native_double_stream_distinct,
    );
    registry.natives_mut().register(
        "duke/util/DoubleStream",
        "boxed",
        "()Ljava/util/stream/Stream;",
        native_double_stream_boxed,
    );

    // OptionalDouble.isPresent()
    registry.natives_mut().register(
        "duke/util/OptionalDouble",
        "isPresent",
        "()Z",
        native_optional_double_is_present,
    );

    // LongStream — reduce(optional), mapToDouble
    registry.natives_mut().register_callback(
        "duke/util/LongStream",
        "reduce",
        "(Ljava/util/function/LongBinaryOperator;)Ljava/util/OptionalLong;",
        native_long_stream_reduce_optional,
    );
    registry.natives_mut().register_callback(
        "duke/util/LongStream",
        "mapToDouble",
        "(Ljava/util/function/LongToDoubleFunction;)Ljava/util/stream/DoubleStream;",
        native_long_stream_map_to_double,
    );

    // Collectors.summingLong + AveragingDouble
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "summingLong",
        "(Ljava/util/function/ToLongFunction;)Ljava/util/stream/Collector;",
        native_collectors_summing_long,
    );
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "averagingDouble",
        "(Ljava/util/function/ToDoubleFunction;)Ljava/util/stream/Collector;",
        native_collectors_averaging_double,
    );
    // Sentinel classes for new collectors
    registry.register(ClassContext {
        class_name: "duke/util/SummingLongCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fn".to_string(),
            descriptor: "Ljava/util/function/ToLongFunction;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    });
    registry.register(ClassContext {
        class_name: "duke/util/AveragingDoubleCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fn".to_string(),
            descriptor: "Ljava/util/function/ToDoubleFunction;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    });

    // ---------------------------------------------------------------------------
    // Phase 56: Collectors.minBy/maxBy, summingDouble, averagingLong,
    //           toUnmodifiableMap, collectingAndThen
    // ---------------------------------------------------------------------------

    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "minBy",
        "(Ljava/util/Comparator;)Ljava/util/stream/Collector;",
        native_collectors_min_by,
    );
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "maxBy",
        "(Ljava/util/Comparator;)Ljava/util/stream/Collector;",
        native_collectors_max_by,
    );
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "summingDouble",
        "(Ljava/util/function/ToDoubleFunction;)Ljava/util/stream/Collector;",
        native_collectors_summing_double,
    );
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "averagingLong",
        "(Ljava/util/function/ToLongFunction;)Ljava/util/stream/Collector;",
        native_collectors_averaging_long,
    );
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "toUnmodifiableMap",
        "(Ljava/util/function/Function;Ljava/util/function/Function;)Ljava/util/stream/Collector;",
        native_collectors_to_unmodifiable_map,
    );
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "collectingAndThen",
        "(Ljava/util/stream/Collector;Ljava/util/function/Function;)Ljava/util/stream/Collector;",
        native_collectors_collecting_and_then,
    );

    // MinByCollector: fields[0] = comparator
    registry.register(ClassContext {
        class_name: "duke/util/MinByCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "comparator".to_string(),
            descriptor: "Ljava/util/Comparator;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    });
    // MaxByCollector: fields[0] = comparator
    registry.register(ClassContext {
        class_name: "duke/util/MaxByCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "comparator".to_string(),
            descriptor: "Ljava/util/Comparator;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    });
    // SummingDoubleCollector: fields[0] = ToDoubleFunction
    registry.register(ClassContext {
        class_name: "duke/util/SummingDoubleCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fn".to_string(),
            descriptor: "Ljava/util/function/ToDoubleFunction;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    });
    // AveragingLongCollector: fields[0] = ToLongFunction
    registry.register(ClassContext {
        class_name: "duke/util/AveragingLongCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "fn".to_string(),
            descriptor: "Ljava/util/function/ToLongFunction;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    });
    // CollectingAndThenCollector: fields[0]=downstream, fields[1]=finisher
    registry.register(ClassContext {
        class_name: "duke/util/CollectingAndThenCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "downstream".to_string(),
                descriptor: "Ljava/util/stream/Collector;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "finisher".to_string(),
                descriptor: "Ljava/util/function/Function;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    });

    // ---- Phase 57: Collectors.reducing sentinels ----

    // ReducingNoIdentityCollector: fields[0] = BinaryOperator
    registry.register(ClassContext {
        class_name: "duke/util/ReducingNoIdentityCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "op".to_string(),
            descriptor: "Ljava/util/function/BinaryOperator;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    });
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "reducing",
        "(Ljava/util/function/BinaryOperator;)Ljava/util/stream/Collector;",
        native_collectors_reducing_no_identity,
    );

    // ReducingCollector: fields[0]=identity, fields[1]=BinaryOperator
    registry.register(ClassContext {
        class_name: "duke/util/ReducingCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "identity".to_string(),
                descriptor: "Ljava/lang/Object;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "op".to_string(),
                descriptor: "Ljava/util/function/BinaryOperator;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    });
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "reducing",
        "(Ljava/lang/Object;Ljava/util/function/BinaryOperator;)Ljava/util/stream/Collector;",
        native_collectors_reducing_with_identity,
    );

    // ReducingMappingCollector: fields[0]=identity, fields[1]=mapper, fields[2]=BinaryOperator
    registry.register(ClassContext {
        class_name: "duke/util/ReducingMappingCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "identity".to_string(),
                descriptor: "Ljava/lang/Object;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "mapper".to_string(),
                descriptor: "Ljava/util/function/Function;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "op".to_string(),
                descriptor: "Ljava/util/function/BinaryOperator;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 3,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
    });
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "reducing",
        "(Ljava/lang/Object;Ljava/util/function/Function;Ljava/util/function/BinaryOperator;)Ljava/util/stream/Collector;",
        native_collectors_reducing_mapping,
    );

    // ---- Phase 57: Stream.iterate 3-arg (Java 9) ----
    registry.natives_mut().register_callback(
        "java/util/stream/Stream",
        "iterate",
        "(Ljava/lang/Object;Ljava/util/function/Predicate;Ljava/util/function/UnaryOperator;)Ljava/util/stream/Stream;",
        native_stream_iterate_predicate,
    );

    // ---- Phase 57: Optional.stream() ----
    registry.natives_mut().register(
        "java/util/Optional",
        "stream",
        "()Ljava/util/stream/Stream;",
        native_optional_stream,
    );

    // ---- Phase 57: ArrayDeque completion ----
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "addFirst",
        "(Ljava/lang/Object;)V",
        native_arraydeque_add_first,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "addLast",
        "(Ljava/lang/Object;)V",
        native_arraydeque_add_last,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "offerFirst",
        "(Ljava/lang/Object;)Z",
        native_arraydeque_offer_first,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "offerLast",
        "(Ljava/lang/Object;)Z",
        native_arraydeque_offer_last,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "peekFirst",
        "()Ljava/lang/Object;",
        native_arraydeque_peek_first,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "peekLast",
        "()Ljava/lang/Object;",
        native_arraydeque_peek_last,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "pollFirst",
        "()Ljava/lang/Object;",
        native_arraydeque_poll_first,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "pollLast",
        "()Ljava/lang/Object;",
        native_arraydeque_poll_last,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "contains",
        "(Ljava/lang/Object;)Z",
        native_arraydeque_contains,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "stream",
        "()Ljava/util/stream/Stream;",
        native_arraydeque_stream,
    );
    registry.natives_mut().register_callback(
        "java/util/ArrayDeque",
        "forEach",
        "(Ljava/util/function/Consumer;)V",
        native_arraydeque_for_each,
    );
    registry.natives_mut().register(
        "java/util/ArrayDeque",
        "clear",
        "()V",
        native_arraydeque_clear,
    );

    // ---- Phase 58: Comparator.comparingDouble ----
    registry.natives_mut().register(
        "java/util/Comparator",
        "comparingDouble",
        "(Ljava/util/function/ToDoubleFunction;)Ljava/util/Comparator;",
        native_comparator_comparing_double,
    );
    // Register reversed() on all key-extractor comparator types
    for class in &[
        "duke/util/ComparingIntComparator",
        "duke/util/ComparingLongComparator",
        "duke/util/ComparingDoubleComparator",
        "duke/util/ThenComparingComparator",
    ] {
        registry.natives_mut().register_callback(
            class,
            "reversed",
            "()Ljava/util/Comparator;",
            native_comparator_reversed,
        );
    }
    registry.register(ClassContext {
        class_name: "duke/util/ComparingDoubleComparator".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "keyExtractor".to_string(),
            descriptor: "Ljava/util/function/ToDoubleFunction;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec!["java/util/Comparator".to_string()],
        bootstrap_methods: Vec::new(),
    });
    registry.natives_mut().register_callback(
        "duke/util/ComparingDoubleComparator",
        "compare",
        "(Ljava/lang/Object;Ljava/lang/Object;)I",
        native_comparing_double_compare,
    );

    // ---- Phase 58: Map.copyOf, Map.entry, Map.ofEntries ----
    registry.natives_mut().register(
        "java/util/Map",
        "copyOf",
        "(Ljava/util/Map;)Ljava/util/Map;",
        native_map_copy_of,
    );
    registry.natives_mut().register(
        "java/util/Map",
        "entry",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/Map$Entry;",
        native_map_entry_factory,
    );
    registry.natives_mut().register(
        "java/util/Map",
        "ofEntries",
        "([Ljava/util/Map$Entry;)Ljava/util/Map;",
        native_map_of_entries,
    );

    // ---- Phase 58: Collections.singletonMap, singleton (set), unmodifiableSet ----
    registry.natives_mut().register(
        "java/util/Collections",
        "singletonMap",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/Map;",
        native_collections_singleton_map,
    );
    registry.natives_mut().register(
        "java/util/Collections",
        "singleton",
        "(Ljava/lang/Object;)Ljava/util/Set;",
        native_collections_singleton_set,
    );
    registry.natives_mut().register(
        "java/util/Collections",
        "unmodifiableSet",
        "(Ljava/util/Set;)Ljava/util/Set;",
        native_collections_unmodifiable_set,
    );

    // ---- Phase 59: Stream.flatMapToInt/Long/Double ----
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "flatMapToInt",
        "(Ljava/util/function/Function;)Ljava/util/stream/IntStream;",
        native_stream_flat_map_to_int,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "flatMapToLong",
        "(Ljava/util/function/Function;)Ljava/util/stream/LongStream;",
        native_stream_flat_map_to_long,
    );
    registry.natives_mut().register_callback(
        "duke/util/Stream",
        "flatMapToDouble",
        "(Ljava/util/function/Function;)Ljava/util/stream/DoubleStream;",
        native_stream_flat_map_to_double,
    );

    // ---- Phase 59: Collectors.toMap (3-arg with merge function) ----
    registry.natives_mut().register_callback(
        "java/util/stream/Collectors",
        "toMap",
        "(Ljava/util/function/Function;Ljava/util/function/Function;Ljava/util/function/BinaryOperator;)Ljava/util/stream/Collector;",
        native_collectors_to_map_merge,
    );

    // ---- Phase 59: forEach on remaining collection types ----
    registry.natives_mut().register_callback(
        "java/util/HashSet",
        "forEach",
        "(Ljava/util/function/Consumer;)V",
        native_hashset_for_each,
    );
    registry.natives_mut().register_callback(
        "java/util/TreeSet",
        "forEach",
        "(Ljava/util/function/Consumer;)V",
        native_treeset_for_each,
    );
    registry.natives_mut().register_callback(
        "java/util/TreeMap",
        "forEach",
        "(Ljava/util/function/BiConsumer;)V",
        native_treemap_for_each,
    );
    registry.natives_mut().register_callback(
        "java/util/LinkedList",
        "forEach",
        "(Ljava/util/function/Consumer;)V",
        native_linked_list_for_each,
    );
    registry.natives_mut().register_callback(
        "java/util/LinkedHashMap",
        "forEach",
        "(Ljava/util/function/BiConsumer;)V",
        native_linkedhashmap_for_each,
    );
    registry.natives_mut().register_callback(
        "java/util/PriorityQueue",
        "forEach",
        "(Ljava/util/function/Consumer;)V",
        native_priorityqueue_for_each,
    );

    // ---- Phase 60: IntStream/LongStream/DoubleStream takeWhile/dropWhile ----
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "takeWhile",
        "(Ljava/util/function/IntPredicate;)Ljava/util/stream/IntStream;",
        native_int_stream_take_while,
    );
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "dropWhile",
        "(Ljava/util/function/IntPredicate;)Ljava/util/stream/IntStream;",
        native_int_stream_drop_while,
    );
    registry.natives_mut().register_callback(
        "duke/util/LongStream",
        "takeWhile",
        "(Ljava/util/function/LongPredicate;)Ljava/util/stream/LongStream;",
        native_long_stream_take_while,
    );
    registry.natives_mut().register_callback(
        "duke/util/LongStream",
        "dropWhile",
        "(Ljava/util/function/LongPredicate;)Ljava/util/stream/LongStream;",
        native_long_stream_drop_while,
    );
    registry.natives_mut().register_callback(
        "duke/util/DoubleStream",
        "takeWhile",
        "(Ljava/util/function/DoublePredicate;)Ljava/util/stream/DoubleStream;",
        native_double_stream_take_while,
    );
    registry.natives_mut().register_callback(
        "duke/util/DoubleStream",
        "dropWhile",
        "(Ljava/util/function/DoublePredicate;)Ljava/util/stream/DoubleStream;",
        native_double_stream_drop_while,
    );

    // ---- Phase 60: Integer/Long/Double compare/max/min ----
    registry.natives_mut().register(
        "java/lang/Integer",
        "compare",
        "(II)I",
        native_integer_compare,
    );
    registry
        .natives_mut()
        .register("java/lang/Integer", "max", "(II)I", native_integer_max);
    registry
        .natives_mut()
        .register("java/lang/Integer", "min", "(II)I", native_integer_min);
    registry
        .natives_mut()
        .register("java/lang/Long", "compare", "(JJ)I", native_long_compare);
    registry
        .natives_mut()
        .register("java/lang/Long", "max", "(JJ)J", native_long_max);
    registry
        .natives_mut()
        .register("java/lang/Long", "min", "(JJ)J", native_long_min);
    registry.natives_mut().register(
        "java/lang/Double",
        "compare",
        "(DD)I",
        native_double_compare,
    );
    registry
        .natives_mut()
        .register("java/lang/Double", "max", "(DD)D", native_double_max);
    registry
        .natives_mut()
        .register("java/lang/Double", "min", "(DD)D", native_double_min);

    // ---- Phase 60: TreeMap.keySet/values/getOrDefault, TreeSet.stream ----
    registry.natives_mut().register(
        "java/util/TreeMap",
        "keySet",
        "()Ljava/util/Set;",
        native_treemap_key_set,
    );
    registry.natives_mut().register(
        "java/util/TreeMap",
        "values",
        "()Ljava/util/Collection;",
        native_treemap_values,
    );
    registry.natives_mut().register(
        "java/util/TreeMap",
        "getOrDefault",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        native_treemap_get_or_default,
    );
    registry.natives_mut().register(
        "java/util/TreeSet",
        "stream",
        "()Ljava/util/stream/Stream;",
        native_treeset_stream,
    );

    // ---- Phase 62: java.time ----
    let localdate_ctx = ClassContext {
        class_name: "java/time/LocalDate".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(localdate_ctx);

    registry.natives_mut().register(
        "java/time/LocalDate",
        "of",
        "(III)Ljava/time/LocalDate;",
        native_localdate_of,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "now",
        "()Ljava/time/LocalDate;",
        native_localdate_now,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "getYear",
        "()I",
        native_localdate_get_year,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "getMonthValue",
        "()I",
        native_localdate_get_month_value,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "getDayOfMonth",
        "()I",
        native_localdate_get_day_of_month,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "plusDays",
        "(J)Ljava/time/LocalDate;",
        native_localdate_plus_days,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "minusDays",
        "(J)Ljava/time/LocalDate;",
        native_localdate_minus_days,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "plusMonths",
        "(J)Ljava/time/LocalDate;",
        native_localdate_plus_months,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "plusYears",
        "(J)Ljava/time/LocalDate;",
        native_localdate_plus_years,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "isBefore",
        "(Ljava/time/chrono/ChronoLocalDate;)Z",
        native_localdate_is_before,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "isAfter",
        "(Ljava/time/chrono/ChronoLocalDate;)Z",
        native_localdate_is_after,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "isEqual",
        "(Ljava/time/chrono/ChronoLocalDate;)Z",
        native_localdate_is_equal,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "toEpochDay",
        "()J",
        native_localdate_to_epoch_day,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "toString",
        "()Ljava/lang/String;",
        native_localdate_to_string,
    );

    let duration_ctx = ClassContext {
        class_name: "java/time/Duration".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(duration_ctx);

    registry.natives_mut().register(
        "java/time/Duration",
        "ofSeconds",
        "(J)Ljava/time/Duration;",
        native_duration_of_seconds,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "ofMinutes",
        "(J)Ljava/time/Duration;",
        native_duration_of_minutes,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "ofHours",
        "(J)Ljava/time/Duration;",
        native_duration_of_hours,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "ofDays",
        "(J)Ljava/time/Duration;",
        native_duration_of_days,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "getSeconds",
        "()J",
        native_duration_get_seconds,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "toSeconds",
        "()J",
        native_duration_to_seconds,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "toMinutes",
        "()J",
        native_duration_to_minutes,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "toHours",
        "()J",
        native_duration_to_hours,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "toDays",
        "()J",
        native_duration_to_days,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "plus",
        "(Ljava/time/Duration;)Ljava/time/Duration;",
        native_duration_plus,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "minus",
        "(Ljava/time/Duration;)Ljava/time/Duration;",
        native_duration_minus,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "isNegative",
        "()Z",
        native_duration_is_negative,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "isZero",
        "()Z",
        native_duration_is_zero,
    );

    let period_ctx = ClassContext {
        class_name: "java/time/Period".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 3,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(period_ctx);

    registry.natives_mut().register(
        "java/time/Period",
        "of",
        "(III)Ljava/time/Period;",
        native_period_of,
    );
    registry.natives_mut().register(
        "java/time/Period",
        "ofDays",
        "(I)Ljava/time/Period;",
        native_period_of_days,
    );
    registry.natives_mut().register(
        "java/time/Period",
        "ofMonths",
        "(I)Ljava/time/Period;",
        native_period_of_months,
    );
    registry.natives_mut().register(
        "java/time/Period",
        "ofYears",
        "(I)Ljava/time/Period;",
        native_period_of_years,
    );
    registry.natives_mut().register(
        "java/time/Period",
        "getYears",
        "()I",
        native_period_get_years,
    );
    registry.natives_mut().register(
        "java/time/Period",
        "getMonths",
        "()I",
        native_period_get_months,
    );
    registry
        .natives_mut()
        .register("java/time/Period", "getDays", "()I", native_period_get_days);
    registry.natives_mut().register(
        "java/time/Period",
        "isNegative",
        "()Z",
        native_period_is_negative,
    );
    registry
        .natives_mut()
        .register("java/time/Period", "isZero", "()Z", native_period_is_zero);

    let instant_ctx = ClassContext {
        class_name: "java/time/Instant".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(instant_ctx);

    registry.natives_mut().register(
        "java/time/Instant",
        "ofEpochSecond",
        "(J)Ljava/time/Instant;",
        native_instant_of_epoch_second,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "ofEpochMilli",
        "(J)Ljava/time/Instant;",
        native_instant_of_epoch_milli,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "getEpochSecond",
        "()J",
        native_instant_get_epoch_second,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "toEpochMilli",
        "()J",
        native_instant_to_epoch_milli,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "isBefore",
        "(Ljava/time/Instant;)Z",
        native_instant_is_before,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "isAfter",
        "(Ljava/time/Instant;)Z",
        native_instant_is_after,
    );

    // ---- Phase 63: java.time.LocalDateTime ----
    let localdatetime_ctx = ClassContext {
        class_name: "java/time/LocalDateTime".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 5,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(localdatetime_ctx);

    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "of",
        "(IIIII)Ljava/time/LocalDateTime;",
        native_localdatetime_of_ymd_hm,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "of",
        "(IIIIII)Ljava/time/LocalDateTime;",
        native_localdatetime_of_ymd_hms,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "of",
        "(Ljava/time/LocalDate;III)Ljava/time/LocalDateTime;",
        native_localdatetime_of_date_hms,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "now",
        "()Ljava/time/LocalDateTime;",
        native_localdatetime_now,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "getYear",
        "()I",
        native_localdatetime_get_year,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "getMonthValue",
        "()I",
        native_localdatetime_get_month_value,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "getDayOfMonth",
        "()I",
        native_localdatetime_get_day_of_month,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "getHour",
        "()I",
        native_localdatetime_get_hour,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "getMinute",
        "()I",
        native_localdatetime_get_minute,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "getSecond",
        "()I",
        native_localdatetime_get_second,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "toLocalDate",
        "()Ljava/time/LocalDate;",
        native_localdatetime_to_local_date,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "isBefore",
        "(Ljava/time/chrono/ChronoLocalDateTime;)Z",
        native_localdatetime_is_before,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "isAfter",
        "(Ljava/time/chrono/ChronoLocalDateTime;)Z",
        native_localdatetime_is_after,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "toString",
        "()Ljava/lang/String;",
        native_localdatetime_to_string,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "plusDays",
        "(J)Ljava/time/LocalDateTime;",
        native_localdatetime_plus_days,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "withHour",
        "(I)Ljava/time/LocalDateTime;",
        native_localdatetime_with_hour,
    );

    // Phase 64 additions
    registry.natives_mut().register(
        "java/lang/String",
        "indent",
        "(I)Ljava/lang/String;",
        native_string_indent,
    );
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "setCharAt",
        "(IC)V",
        native_stringbuilder_set_char_at,
    );
    registry.natives_mut().register(
        "java/util/Collections",
        "disjoint",
        "(Ljava/util/Collection;Ljava/util/Collection;)Z",
        native_collections_disjoint,
    );
    registry.natives_mut().register_callback(
        "java/util/HashMap",
        "computeIfPresent",
        "(Ljava/lang/Object;Ljava/util/function/BiFunction;)Ljava/lang/Object;",
        native_hashmap_compute_if_present,
    );
}
