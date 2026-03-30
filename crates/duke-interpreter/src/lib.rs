//! Switch-dispatch JVM bytecode interpreter for Duke Phase 4.
//!
//! Executes decoded instruction streams for methods containing integer, long,
//! float, and double arithmetic, control flow, and local variables.  Heap
//! allocation, field access, and method invocation are not yet implemented.

/// Core execution context types for methods and classes.
pub mod context;
/// Repositories for loaded classes and registered native methods.
pub mod registry;
mod threading;

pub use context::*;
pub use registry::*;
pub mod engine;
pub use engine::*;

use std::io::Write;

use duke_runtime::{Slot, VmError, VmResult};

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
    registry.natives_mut().register(
        "java/lang/Class",
        "getName",
        "()Ljava/lang/String;",
        native_class_get_name,
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
        ],
        static_fields: Vec::new(),
        instance_field_count: 5,
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
    registry.natives_mut().register_callback(
        "java/lang/reflect/Method",
        "invoke",
        "(Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;",
        native_reflect_method_invoke,
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
        ],
        static_fields: Vec::new(),
        instance_field_count: 5,
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
    registry
        .natives_mut()
        .register("java/lang/Thread", "start", "()V", native_thread_start);
    registry
        .natives_mut()
        .register("java/lang/Thread", "join", "()V", native_thread_join);
    registry
        .natives_mut()
        .register("java/lang/Thread", "sleep", "(J)V", native_thread_sleep);

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
        ],
        static_fields: vec![Slot::Int(i32::MAX), Slot::Int(i32::MIN)],
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
        "compareTo",
        "(Ljava/lang/Object;)I",
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
        ],
        static_fields: vec![Slot::Long(i64::MAX), Slot::Long(i64::MIN)],
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
        "compareTo",
        "(Ljava/lang/Object;)I",
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
        ],
        static_fields: vec![
            Slot::Double(f64::MAX),
            Slot::Double(5e-324_f64),
            Slot::Double(f64::NAN),
            Slot::Double(f64::INFINITY),
            Slot::Double(f64::NEG_INFINITY),
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

    // java/lang/Float — boxed float with value field
    let float_ctx = ClassContext {
        class_name: "java/lang/Float".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "value".to_string(),
            descriptor: "F".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
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

    // java/lang/Boolean — static utility
    let boolean_ctx = ClassContext {
        class_name: "java/lang/Boolean".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
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
        fields: vec![FieldEntry {
            name: "value".to_string(),
            descriptor: "C".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
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
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
    };
    registry.register(hashset_ctx);
    registry
        .natives_mut()
        .register("java/util/HashSet", "<init>", "()V", native_hashset_init);
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

fn native_println_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let string_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => {
            writeln!(out, "null").ok();
            return Ok(None);
        }
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let obj = heap.get(string_ref)?;
    let text = obj.string_value.as_deref().unwrap_or("null");
    writeln!(out, "{text}").ok();
    Ok(None)
}

fn native_println_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    writeln!(out, "{val}").ok();
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
fn native_println_void(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    writeln!(out).ok();
    Ok(None)
}

fn path_from_string_slot(
    args: &[Slot],
    idx: usize,
    heap: &duke_gc::Heap,
) -> VmResult<std::path::PathBuf> {
    let path_ref = match args.get(idx) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let path = heap
        .get(path_ref)?
        .string_value
        .clone()
        .ok_or(VmError::NullPointerException)?;
    Ok(std::path::PathBuf::from(path))
}

fn string_value_from_ref(heap: &duke_gc::Heap, string_ref: u64) -> VmResult<String> {
    heap.get(string_ref)?
        .string_value
        .clone()
        .ok_or(VmError::NullPointerException)
}

fn file_path_from_ref(file_ref: u64, heap: &duke_gc::Heap) -> VmResult<std::path::PathBuf> {
    let path_ref = match heap.get(file_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    Ok(std::path::PathBuf::from(string_value_from_ref(
        heap, path_ref,
    )?))
}

fn file_path_from_this(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<std::path::PathBuf> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    file_path_from_ref(this_ref, heap)
}

fn native_file_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let path_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let file_obj = heap.get_mut(this_ref)?;
    let Some(path_field) = file_obj.fields.first_mut() else {
        return Err(VmError::InvalidRef { address: this_ref });
    };
    *path_field = path_slot;
    Ok(None)
}

fn native_file_exists(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let path = file_path_from_this(args, heap)?;
    Ok(Some(Slot::Int(i32::from(path.exists()))))
}

fn native_file_is_file(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let path = file_path_from_this(args, heap)?;
    Ok(Some(Slot::Int(i32::from(path.is_file()))))
}

fn native_file_is_directory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let path = file_path_from_this(args, heap)?;
    Ok(Some(Slot::Int(i32::from(path.is_dir()))))
}

fn file_stream_id_from_this(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<i32> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => Err(VmError::JavaException {
            class_name: "java/io/IOException".to_string(),
        }),
    }
}

fn native_file_input_stream_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let path = path_from_string_slot(args, 1, heap)?;
    let file_id = heap.open_host_input_file(&path)?;
    let stream_obj = heap.get_mut(this_ref)?;
    let Some(fd_field) = stream_obj.fields.first_mut() else {
        return Err(VmError::InvalidRef { address: this_ref });
    };
    *fd_field = Slot::Int(file_id);
    Ok(None)
}

fn native_file_input_stream_read(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let file_id = file_stream_id_from_this(args, heap)?;
    Ok(Some(Slot::Int(heap.read_host_file_byte(file_id)?)))
}

fn native_file_input_stream_read_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let file_id = file_stream_id_from_this(args, heap)?;
    let array_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let len = heap.get(array_ref)?.fields.len();
    if len == 0 {
        return Ok(Some(Slot::Int(0)));
    }

    let mut count = 0_usize;
    for idx in 0..len {
        let next = heap.read_host_file_byte(file_id)?;
        if next < 0 {
            break;
        }
        heap.get_mut(array_ref)?.fields[idx] = Slot::Int(next);
        count += 1;
    }

    if count == 0 {
        Ok(Some(Slot::Int(-1)))
    } else {
        Ok(Some(Slot::Int(i32::try_from(count).unwrap_or(i32::MAX))))
    }
}

fn native_file_input_stream_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let file_id = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) => *id,
        _ => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
        }
    };
    heap.close_host_file(file_id);
    let stream_obj = heap.get_mut(this_ref)?;
    let Some(fd_field) = stream_obj.fields.first_mut() else {
        return Err(VmError::InvalidRef { address: this_ref });
    };
    *fd_field = Slot::Int(0);
    Ok(None)
}

fn native_file_output_stream_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let path = path_from_string_slot(args, 1, heap)?;
    let file_id = heap.open_host_output_file(&path)?;
    let stream_obj = heap.get_mut(this_ref)?;
    let Some(fd_field) = stream_obj.fields.first_mut() else {
        return Err(VmError::InvalidRef { address: this_ref });
    };
    *fd_field = Slot::Int(file_id);
    Ok(None)
}

fn native_file_output_stream_write(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let file_id = file_stream_id_from_this(args, heap)?;
    let value = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    heap.write_host_file_byte(file_id, value)?;
    Ok(None)
}

fn native_file_output_stream_write_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let file_id = file_stream_id_from_this(args, heap)?;
    let array_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let bytes = heap.get(array_ref)?.fields.clone();
    for byte in bytes {
        let Slot::Int(value) = byte else {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        };
        heap.write_host_file_byte(file_id, value)?;
    }
    Ok(None)
}

fn native_file_output_stream_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let file_id = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) => *id,
        _ => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".to_string(),
            });
        }
    };
    heap.close_host_file(file_id);
    let stream_obj = heap.get_mut(this_ref)?;
    let Some(fd_field) = stream_obj.fields.first_mut() else {
        return Err(VmError::InvalidRef { address: this_ref });
    };
    *fd_field = Slot::Int(0);
    Ok(None)
}

// ── Networking natives ────────────────────────────────────────────────────

/// Native: `ServerSocket.<init>(int port)` — binds to 0.0.0.0:{port}.
fn native_server_socket_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let port = match args.get(1) {
        Some(Slot::Int(p)) => *p,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let addr = format!("0.0.0.0:{port}");
    let server_id = heap.bind_server_socket(&addr)?;
    let actual_port = heap.server_socket_local_port(server_id)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() < 2 {
        return Err(VmError::InvalidRef { address: this_ref });
    }
    obj.fields[0] = Slot::Int(server_id);
    obj.fields[1] = Slot::Int(actual_port);
    Ok(None)
}

/// Native: `ServerSocket.accept()` — blocks until a client connects, returns a Socket.
fn native_server_socket_accept(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let server_fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let (reader_id, writer_id) = heap.accept_connection(server_fd)?;
    // Allocate a new Socket object with fdRead=reader_id, fdWrite=writer_id
    let socket_ref = heap.allocate("java/net/Socket".to_string(), 2);
    heap.get_mut(socket_ref)?.fields[0] = Slot::Int(reader_id);
    heap.get_mut(socket_ref)?.fields[1] = Slot::Int(writer_id);
    Ok(Some(Slot::Reference(Some(socket_ref))))
}

/// Native: `ServerSocket.getLocalPort()` — returns the bound port.
fn native_server_socket_get_local_port(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(port)) => Ok(Some(Slot::Int(*port))),
        _ => Err(VmError::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
}

/// Native: `ServerSocket.close()` — closes the OS listener and zeros the fd field.
fn native_server_socket_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        Some(Slot::Int(_)) => return Ok(None), // already closed — idempotent
        _ => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    heap.close_host_file(fd);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    obj.fields[1] = Slot::Int(0); // also zero cached port so getLocalPort() returns 0 after close
    Ok(None)
}

/// Native: `Socket.<init>(String host, int port)` — connects to host:port.
fn native_socket_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let host = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap
            .get(*r)?
            .string_value
            .clone()
            .ok_or(VmError::NullPointerException)?,
        _ => return Err(VmError::NullPointerException),
    };
    let port = match args.get(2) {
        Some(Slot::Int(p)) => *p,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let addr = format!("{host}:{port}");
    let (reader_id, writer_id) = heap.connect_socket(&addr)?;
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() < 2 {
        return Err(VmError::InvalidRef { address: this_ref });
    }
    obj.fields[0] = Slot::Int(reader_id);
    obj.fields[1] = Slot::Int(writer_id);
    Ok(None)
}

/// Native: `Socket.getInputStream()` — allocates a `SocketInputStream` wrapping `fdRead`.
fn native_socket_get_input_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let fd_read = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let stream_ref = heap.allocate("duke/net/SocketInputStream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(fd_read);
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `Socket.getOutputStream()` — allocates a `SocketOutputStream` wrapping `fdWrite`.
fn native_socket_get_output_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let fd_write = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(id)) if *id > 0 => *id,
        _ => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let stream_ref = heap.allocate("duke/net/SocketOutputStream".to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(fd_write);
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

/// Native: `Socket.close()` — closes both OS handles (fdRead and fdWrite).
fn native_socket_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let fd_read = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) => *id,
        _ => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let fd_write = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Int(id)) => *id,
        _ => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    heap.close_host_file(fd_read);
    heap.close_host_file(fd_write);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    obj.fields[1] = Slot::Int(0);
    Ok(None)
}

// ── ZIP / JAR natives ──────────────────────────────────────────────────

/// Native: `ZipFile.<init>(String)` — open and index a ZIP/JAR archive.
fn native_zip_file_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let path_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let path_str = heap
        .get(path_ref)?
        .string_value
        .as_deref()
        .ok_or(VmError::NullPointerException)?
        .to_string();
    let fd = heap.open_host_zip(std::path::Path::new(&path_str))?;
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(fd);
    Ok(None)
}

/// Native: `ZipFile.getEntry(String) -> ZipEntry` — look up an entry by name.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn native_zip_file_get_entry(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let name_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) => *id,
        _ => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let entry_name = heap
        .get(name_ref)?
        .string_value
        .as_deref()
        .ok_or(VmError::NullPointerException)?
        .to_string();
    let info = heap.zip_get_entry_info(fd, &entry_name)?;
    let Some(info) = info else {
        return Ok(Some(Slot::Reference(None)));
    };
    // Allocate a ZipEntry HeapObject with 6 fields.
    let name_heap_ref = heap.allocate_string(info.name);
    let entry_ref = heap.allocate("java/util/zip/ZipEntry".to_string(), 6);
    let entry_obj = heap.get_mut(entry_ref)?;
    entry_obj.fields[0] = Slot::Reference(Some(name_heap_ref));
    entry_obj.fields[1] = Slot::Int(info.compressed_size as i32);
    entry_obj.fields[2] = Slot::Int((info.compressed_size >> 32) as i32);
    entry_obj.fields[3] = Slot::Int(info.uncompressed_size as i32);
    entry_obj.fields[4] = Slot::Int((info.uncompressed_size >> 32) as i32);
    entry_obj.fields[5] = Slot::Int(i32::from(info.compression_method));
    Ok(Some(Slot::Reference(Some(entry_ref))))
}

/// Native: `ZipFile.getInputStream(ZipEntry) -> InputStream`
fn native_zip_file_get_input_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let entry_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) => *id,
        _ => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    // Get entry name from the ZipEntry object.
    let name_slot_ref = match heap.get(entry_ref)?.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let entry_name = heap
        .get(name_slot_ref)?
        .string_value
        .as_deref()
        .ok_or(VmError::NullPointerException)?
        .to_string();
    // Decompress the entry and wrap in a ByteBuffer.
    let data = heap.zip_read_entry(fd, &entry_name)?;
    let buf_fd = heap.open_host_byte_buffer(data);
    let is_ref = heap.allocate("duke/zip/ByteBufferInputStream".to_string(), 1);
    let is_obj = heap.get_mut(is_ref)?;
    is_obj.fields[0] = Slot::Int(buf_fd);
    Ok(Some(Slot::Reference(Some(is_ref))))
}

/// Native: `ZipFile.close()`
fn native_zip_file_close(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) => *id,
        _ => return Ok(None),
    };
    heap.close_host_file(fd);
    let obj = heap.get_mut(this_ref)?;
    obj.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `ZipFile.size() -> int`
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn native_zip_file_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let fd = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(id)) => *id,
        _ => {
            return Err(VmError::JavaException {
                class_name: "java/io/IOException".into(),
            });
        }
    };
    let count = heap.zip_entry_count(fd)?;
    Ok(Some(Slot::Int(count as i32)))
}

/// Native: `ZipEntry.getName() -> String`
fn native_zip_entry_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    Ok(Some(heap.get(this_ref)?.fields[0]))
}

/// Native: `ZipEntry.getCompressedSize() -> long`
fn native_zip_entry_get_compressed_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let fields = &heap.get(this_ref)?.fields;
    let lo = match fields[1] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let hi = match fields[2] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let val = (i64::from(hi) << 32) | (i64::from(lo) & 0xFFFF_FFFF);
    Ok(Some(Slot::Long(val)))
}

/// Native: `ZipEntry.getSize() -> long`
fn native_zip_entry_get_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let fields = &heap.get(this_ref)?.fields;
    let lo = match fields[3] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let hi = match fields[4] {
        Slot::Int(v) => v,
        _ => 0,
    };
    let val = (i64::from(hi) << 32) | (i64::from(lo) & 0xFFFF_FFFF);
    Ok(Some(Slot::Long(val)))
}

/// Native: `ZipEntry.getMethod() -> int`
fn native_zip_entry_get_method(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    Ok(Some(heap.get(this_ref)?.fields[5]))
}

/// Native: `String.length()` — returns string length as int.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn native_string_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get(this_ref)?;
    let len = obj.string_value.as_ref().map_or(0, String::len);
    Ok(Some(Slot::Int(len as i32)))
}

/// Native: `String.equals(Object)` — compares string content.
fn native_string_equals(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let other_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Ok(Some(Slot::Int(0))),
    };
    // Fetch objects from heap in one go to keep borrows short
    let this_obj = heap.get(this_ref)?;
    let other_obj = heap.get(other_ref)?;
    let this_str = this_obj.string_value.as_deref().unwrap_or_default();
    let other_str = other_obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(this_str == other_str))))
}

/// Native: `String.charAt(int)` — returns char at index as int.
#[allow(clippy::cast_sign_loss)]
fn native_string_char_at(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let index = match args.get(1) {
        Some(Slot::Int(i)) => *i,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let obj = heap.get(this_ref)?;
    let s = obj.string_value.as_deref().unwrap_or("");
    let ch = s
        .chars()
        .nth(index as usize)
        .ok_or(VmError::ArrayIndexOutOfBounds {
            index,
            length: s.len(),
        })?;
    Ok(Some(Slot::Int(ch as i32)))
}

/// Native: `Object.hashCode()` — returns heap address as hash.
fn native_object_hashcode(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Some(Slot::Reference(Some(r))) => Ok(Some(Slot::Int(*r as i32))),
        _ => Err(VmError::NullPointerException),
    }
}

/// Native: `Object.toString()` — delegates to `heap_object_to_string` so String,
/// boxed primitives, and opaque objects all produce the correct Java representation.
fn native_object_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap_object_to_string(heap.get(this_ref)?, this_ref);
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Object.clone()` — shallow-copies a heap object.
fn native_object_clone(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get(this_ref)?;
    let cloned_class = obj.class_name.clone();
    let cloned_fields = obj.fields.clone();
    let cloned_string = obj.string_value.clone();
    let new_ref = heap.allocate(cloned_class, 0);
    let dest = heap.get_mut(new_ref)?;
    dest.fields = cloned_fields;
    dest.string_value = cloned_string;
    Ok(Some(Slot::Reference(Some(new_ref))))
}

/// `Throwable.addSuppressed(Throwable suppressed)V`
///
/// No-op stub. Control flow is handled entirely by the bytecode desugaring —
/// `addSuppressed` only affects what `getSuppressed()` returns, which is not
/// yet implemented. Suppressed exception is silently dropped.
///
/// Signature: `args[0]` = this (Throwable), `args[1]` = suppressed (Throwable)
#[allow(clippy::unnecessary_wraps)]
fn native_throwable_add_suppressed(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _stdout: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(None)
}

/// Native: `Enum.<init>(Ljava/lang/String;I)V` — stores name + ordinal.
/// args: `[this_ref, name_ref, ordinal_int]`
fn native_enum_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let name_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let ordinal = match args.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.len() >= 2 {
        obj.fields[0] = name_slot;
        obj.fields[1] = Slot::Int(ordinal);
    }
    Ok(None)
}

/// Native: `Enum.ordinal()I`
fn native_enum_ordinal(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get(this_ref)?;
    match obj.fields.get(1) {
        Some(Slot::Int(v)) => Ok(Some(Slot::Int(*v))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `Enum.name()Ljava/lang/String;`
fn native_enum_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get(this_ref)?;
    match obj.fields.first() {
        Some(slot @ Slot::Reference(_)) => Ok(Some(*slot)),
        _ => Ok(Some(Slot::Reference(None))),
    }
}

/// Native: `Enum.valueOf(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;`
/// Searches heap for enum constants of the given class matching the name.
#[allow(clippy::cast_possible_truncation)]
fn native_enum_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let class_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let name_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let target_name = heap.get(name_ref)?.string_value.clone().unwrap_or_default();
    let enum_class_name = heap
        .get(class_ref)?
        .string_value
        .clone()
        .unwrap_or_default();

    let obj_count = heap.len();
    for i in 0..obj_count {
        let obj = heap.get(i as u64)?;
        if obj.class_name == enum_class_name
            && obj.fields.len() >= 2
            && let Some(Slot::Reference(Some(name_r))) = obj.fields.first()
            && let Ok(name_obj) = heap.get(*name_r)
            && name_obj.string_value.as_deref() == Some(target_name.as_str())
        {
            return Ok(Some(Slot::Reference(Some(i as u64))));
        }
    }

    Err(VmError::JavaException {
        class_name: "java/lang/IllegalArgumentException".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Reflection natives
// ---------------------------------------------------------------------------

fn native_class_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let class_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let name_ref = heap.allocate_string(internal_name_to_binary_name(&internal_name));
    Ok(Some(Slot::Reference(Some(name_ref))))
}

fn native_class_for_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let name_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let binary_name = heap
        .get(name_ref)?
        .string_value
        .clone()
        .ok_or(VmError::NullPointerException)?;
    let internal_name = binary_name_to_internal_name(&binary_name);
    match ops.ensure_loaded(&internal_name) {
        Ok(()) => {
            let class_ref = allocate_class_object(heap, &internal_name)?;
            Ok(Some(Slot::Reference(Some(class_ref))))
        }
        Err(VmError::ClassNotFound { .. }) => Err(VmError::JavaException {
            class_name: "java/lang/ClassNotFoundException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

fn native_class_get_declared_methods(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&internal_name)?;
    let method_refs = reflected
        .methods
        .into_iter()
        .filter(|method| method.name != "<init>" && method.name != "<clinit>")
        .map(|method| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Method",
                &reflected.internal_name,
                &method.name,
                &method.descriptor,
                method.is_public,
                method.is_static,
            )
        })
        .collect::<VmResult<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Method;", &method_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

fn native_class_get_declared_fields(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let class_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let internal_name = class_internal_name_from_ref(heap, class_ref)?;
    let reflected = ops.inspect_class(&internal_name)?;
    let field_refs = reflected
        .fields
        .into_iter()
        .map(|field| {
            allocate_reflection_member_object(
                heap,
                "java/lang/reflect/Field",
                &reflected.internal_name,
                &field.name,
                &field.descriptor,
                field.is_public,
                field.is_static,
            )
        })
        .collect::<VmResult<Vec<_>>>()?;
    let array_ref = allocate_reference_array(heap, "[Ljava/lang/reflect/Field;", &field_refs)?;
    Ok(Some(Slot::Reference(Some(array_ref))))
}

fn native_reflect_method_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let method_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    Ok(Some(reflection_member_name_slot(heap, method_ref)?))
}

fn native_reflect_field_get_name(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let field_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    Ok(Some(reflection_member_name_slot(heap, field_ref)?))
}

fn native_reflect_method_invoke(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    let method_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let target_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let invoke_arg_slots =
        reflection_array_elements(heap, args.get(2).copied().unwrap_or(Slot::Reference(None)))?;
    let method = reflected_method_handle(heap, method_ref)?;

    if !method.is_public {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalAccessException".to_string(),
        });
    }

    let invoke_args = build_reflection_invoke_args(
        heap,
        target_slot,
        &method.descriptor,
        invoke_arg_slots,
        method.is_static,
    )?;

    ops.ensure_loaded(&method.declaring_internal_name)?;
    match ops.invoke(
        heap,
        output,
        &method.declaring_internal_name,
        &method.method_name,
        &method.descriptor,
        invoke_args,
    ) {
        Ok(result) => Ok(Some(box_reflection_return_value(
            heap,
            descriptor_return_type(&method.descriptor),
            result,
        )?)),
        Err(VmError::JavaException { .. }) => Err(VmError::JavaException {
            class_name: "java/lang/reflect/InvocationTargetException".to_string(),
        }),
        Err(err) => Err(err),
    }
}

/// Native: `String.valueOf(int)` — static method, returns string of int.
fn native_string_value_of_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let s = val.to_string();
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `PrintStream.print(String)` — no newline.
fn native_print_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let string_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => {
            write!(out, "null").ok();
            return Ok(None);
        }
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let obj = heap.get(string_ref)?;
    let text = obj.string_value.as_deref().unwrap_or("null");
    write!(out, "{text}").ok();
    Ok(None)
}

/// Native: `PrintStream.print(int)` — no newline.
#[allow(clippy::unnecessary_wraps)]
fn native_print_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    write!(out, "{val}").ok();
    Ok(None)
}

// ---------------------------------------------------------------------------
// println overloads (long, float, double, boolean, char, object)
// ---------------------------------------------------------------------------

fn native_println_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    writeln!(out, "{val}").ok();
    Ok(None)
}

fn native_println_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Float(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Float",
                got: "other",
            });
        }
    };
    writeln!(out, "{val}").ok();
    Ok(None)
}

fn native_println_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    writeln!(out, "{val}").ok();
    Ok(None)
}

fn native_println_boolean(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v != 0,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int(boolean)",
                got: "other",
            });
        }
    };
    writeln!(out, "{val}").ok();
    Ok(None)
}

fn native_println_char(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Int(v)) => char::from_u32((*v).cast_unsigned()).unwrap_or('?'),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int(char)",
                got: "other",
            });
        }
    };
    writeln!(out, "{val}").ok();
    Ok(None)
}

#[allow(clippy::cast_possible_wrap)]
/// Convert a heap object to its Java display string.
///
/// Checks `string_value` first (handles String/StringBuilder).
/// For boxed primitives, extracts the stored value from `fields[0]`.
/// Falls back to `class_name@hex_ref` for opaque objects.
fn heap_object_to_string(obj: &duke_gc::HeapObject, obj_ref: u64) -> String {
    if let Some(s) = &obj.string_value {
        return s.clone();
    }
    match obj.class_name.as_str() {
        "java/lang/Integer" => {
            if let Some(Slot::Int(v)) = obj.fields.first() {
                return v.to_string();
            }
        }
        "java/lang/Long" => {
            if let Some(Slot::Long(v)) = obj.fields.first() {
                return v.to_string();
            }
        }
        "java/lang/Double" => {
            if let Some(Slot::Double(v)) = obj.fields.first() {
                return format_java_double(*v);
            }
        }
        "java/lang/Float" => {
            if let Some(Slot::Float(v)) = obj.fields.first() {
                return format_java_float(*v);
            }
        }
        "java/lang/Boolean" => {
            return match obj.fields.first() {
                Some(Slot::Int(v)) if *v != 0 => "true".to_string(),
                _ => "false".to_string(),
            };
        }
        "java/lang/Character" => {
            if let Some(Slot::Int(v)) = obj.fields.first()
                && let Some(c) = char::from_u32((*v).cast_unsigned())
            {
                return c.to_string();
            }
        }
        _ => {}
    }
    format!("{}@{:x}", obj.class_name, obj_ref)
}

fn native_println_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.get(1) {
        Some(Slot::Reference(Some(r))) => {
            let s = heap_object_to_string(heap.get(*r)?, *r);
            writeln!(out, "{s}").ok();
        }
        Some(Slot::Reference(None)) => {
            writeln!(out, "null").ok();
        }
        _ => {
            writeln!(out, "<unknown>").ok();
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// print overloads (long, float, double, boolean, char, object)
// ---------------------------------------------------------------------------

fn native_print_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    write!(out, "{val}").ok();
    Ok(None)
}

fn native_print_float(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Float(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Float",
                got: "other",
            });
        }
    };
    write!(out, "{val}").ok();
    Ok(None)
}

fn native_print_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    write!(out, "{val}").ok();
    Ok(None)
}

fn native_print_boolean(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v != 0,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int(boolean)",
                got: "other",
            });
        }
    };
    write!(out, "{val}").ok();
    Ok(None)
}

fn native_print_char(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.get(1) {
        Some(Slot::Int(v)) => char::from_u32((*v).cast_unsigned()).unwrap_or('?'),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int(char)",
                got: "other",
            });
        }
    };
    write!(out, "{val}").ok();
    Ok(None)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn native_print_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.get(1) {
        Some(Slot::Reference(Some(r))) => {
            let s = heap_object_to_string(heap.get(*r)?, *r);
            write!(out, "{s}").ok();
        }
        Some(Slot::Reference(None)) => {
            write!(out, "null").ok();
        }
        _ => {
            write!(out, "<unknown>").ok();
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// System.exit
// ---------------------------------------------------------------------------

fn native_system_exit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let code = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 1,
    };
    Err(VmError::SystemExit { code })
}

pub(crate) const THREAD_TARGET_SLOT: usize = 0;
pub(crate) const THREAD_ID_SLOT: usize = 1;

fn native_thread_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let this = heap.get_mut(this_ref)?;
    this.fields[THREAD_TARGET_SLOT] = Slot::Reference(None);
    this.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    Ok(None)
}

fn native_thread_init_runnable(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let target = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let this = heap.get_mut(this_ref)?;
    this.fields[THREAD_TARGET_SLOT] = target;
    this.fields[THREAD_ID_SLOT] = Slot::Int(-1);
    Ok(None)
}

fn native_thread_start(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let thread_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    control.request(NativeThreadAction::Start { thread_ref });
    Ok(None)
}

fn native_thread_join(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let thread_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let thread = heap.get(thread_ref)?;
    let Some(Slot::Int(thread_id)) = thread.fields.get(THREAD_ID_SLOT) else {
        return Ok(None);
    };
    if *thread_id >= 0 {
        control.request(NativeThreadAction::Join {
            thread_id: *thread_id,
        });
    }
    Ok(None)
}

fn native_thread_sleep(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let millis = match args.first() {
        Some(Slot::Long(value)) => *value,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "long",
                got: "other",
            });
        }
    };
    let millis = u64::try_from(millis.max(0)).unwrap_or(0);
    control.request(NativeThreadAction::Sleep(std::time::Duration::from_millis(
        millis,
    )));
    Ok(None)
}

/// Native: `String.substring(int)` — substring from begin to end.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn native_string_substring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };

    let begin = match args.get(1) {
        Some(Slot::Int(v)) => *v as usize,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let sub = {
        let obj = heap.get(this_ref)?;
        let s = obj.string_value.as_deref().unwrap_or_default();
        if begin > s.len() {
            return Err(VmError::ArrayIndexOutOfBounds {
                index: i32::try_from(begin).unwrap_or(i32::MAX),
                length: s.len(),
            });
        }
        s.chars().skip(begin).collect::<String>()
    };

    let r = heap.allocate_string(sub);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.substring(int, int)` — substring from begin to end (exclusive).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn native_string_substring_range(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };

    let begin = match args.get(1) {
        Some(Slot::Int(v)) => *v as usize,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let end = match args.get(2) {
        Some(Slot::Int(v)) => *v as usize,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let sub = {
        let obj = heap.get(this_ref)?;
        let s = obj.string_value.as_deref().unwrap_or_default();
        if begin > end || end > s.len() {
            return Err(VmError::ArrayIndexOutOfBounds {
                index: i32::try_from(end).unwrap_or(i32::MAX),
                length: s.len(),
            });
        }
        s.chars().skip(begin).take(end - begin).collect::<String>()
    };

    let r = heap.allocate_string(sub);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.indexOf(String)` — find first occurrence of target.
fn native_string_indexof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };

    let target_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    // Fetch objects from heap in one go to keep borrows short
    let this_obj = heap.get(this_ref)?;
    let target_obj = heap.get(target_ref)?;
    let s = this_obj.string_value.as_deref().unwrap_or_default();
    let target = target_obj.string_value.as_deref().unwrap_or_default();

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let result = s.find(target).map_or(-1, |i| i as i32);
    Ok(Some(Slot::Int(result)))
}

/// Native: `String.contains(CharSequence)` — check if string contains target.
fn native_string_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };

    let target_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    // Fetch objects from heap in one go to keep borrows short
    let this_obj = heap.get(this_ref)?;
    let target_obj = heap.get(target_ref)?;
    let s = this_obj.string_value.as_deref().unwrap_or_default();
    let target = target_obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.contains(target)))))
}

/// Native: `String.isEmpty()` — check if string is empty.
#[allow(clippy::unnecessary_wraps)]
fn native_string_isempty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get(this_ref)?;
    let s = obj.string_value.as_deref().unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.is_empty()))))
}

// Dispatch string constants used by the callback-based sort chain.
const COMPARE_TO_METHOD: &str = "compareTo";
const COMPARE_TO_OBJECT_DESC: &str = "(Ljava/lang/Object;)I";
const SORT_COMPARATOR_DESC: &str = "(Ljava/util/Comparator;)V";

/// Maps a `std::cmp::Ordering` to the Java `compareTo` convention: -1 / 0 / 1.
///
/// Used by all boxed-type `compareTo` natives to return a consistent,
/// sign-correct value without relying on `Ordering`'s internal discriminant.
#[inline]
const fn ordering_to_int(o: std::cmp::Ordering) -> i32 {
    match o {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Native: `String.compareTo(String)` — delegates to the Object overload.
fn native_string_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
    control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    native_string_compareto_object(args, heap, out, control)
}

/// Native: `String.compareTo(Object)` — lexicographic comparison via Object descriptor.
fn native_string_compareto_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let str_val = |s: &Slot| -> VmResult<String> {
        match s {
            Slot::Reference(Some(r)) => Ok(heap.get(*r)?.string_value.clone().unwrap_or_default()),
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => str_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = match args.get(1) {
        Some(s) => str_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    Ok(Some(Slot::Int(ordering_to_int(a.as_str().cmp(b.as_str())))))
}

/// Native: `String.startsWith(String)` — check if string starts with prefix.
fn native_string_startswith(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let prefix_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let prefix = heap
        .get(prefix_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.starts_with(&prefix)))))
}

/// Native: `String.endsWith(String)` — check if string ends with suffix.
fn native_string_endswith(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let suffix_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let suffix = heap
        .get(suffix_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    Ok(Some(Slot::Int(i32::from(s.ends_with(&suffix)))))
}

/// Native: `String.trim()` — remove leading and trailing whitespace.
fn native_string_trim(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let trimmed = s.trim().to_string();
    let r = heap.allocate_string(trimmed);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.toCharArray()` — convert string to char array.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn native_string_tochararray(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let chars: Vec<char> = s.chars().collect();
    let arr_ref = heap.allocate("[C".to_string(), chars.len());
    for (i, &c) in chars.iter().enumerate() {
        heap.get_mut(arr_ref).unwrap().fields[i] = Slot::Int(c as i32);
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

// ---- Integer natives ----

/// Native: `Integer.parseInt(String)` — parses string to int.
fn native_integer_parseint(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: i32 = s.trim().parse().map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(Some(Slot::Int(val)))
}

/// Native: `Integer.valueOf(int)` — boxes int into Integer object.
fn native_integer_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let r = heap.allocate("java/lang/Integer".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.intValue()` — unboxes Integer to int.
fn native_integer_intvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

/// Native: `Integer.toString(int)` — static, converts int to String.
fn native_integer_tostring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Integer.compareTo(Object)` — compares two boxed Integers.
fn native_integer_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let int_val = |s: &Slot| -> VmResult<i32> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Int(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => int_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = match args.get(1) {
        Some(s) => int_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

// ---- String.valueOf overloads ----

/// Native: `String.valueOf(long)` — converts long to String.
fn native_string_value_of_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(double)` — converts double to String.
fn native_string_value_of_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(float)` — converts float to String.
fn native_string_value_of_float(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Float(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Float",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(boolean)` — converts boolean to String.
fn native_string_value_of_boolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v != 0,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int(boolean)",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(if val { "true" } else { "false" }.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(char)` — converts char to String.
fn native_string_value_of_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => char::from_u32((*v).cast_unsigned()).unwrap_or('?'),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int(char)",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.valueOf(Object)` — converts Object to String.
fn native_string_value_of_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let s = heap_object_to_string(heap.get(*r)?, *r);
            let r = heap.allocate_string(s);
            Ok(Some(Slot::Reference(Some(r))))
        }
        Some(Slot::Reference(None)) => {
            let r = heap.allocate_string("null".to_string());
            Ok(Some(Slot::Reference(Some(r))))
        }
        _ => Err(VmError::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

// ---- String.concat ----

/// Native: `String.concat(String)` — concatenates two strings.
fn native_string_concat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s1 = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let other_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s2 = heap
        .get(other_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let r = heap.allocate_string(format!("{s1}{s2}"));
    Ok(Some(Slot::Reference(Some(r))))
}

/// Formats a single boxed slot value using the given format specifier.
fn format_arg(
    spec: char,
    precision: Option<usize>,
    slot: &Slot,
    heap: &duke_gc::Heap,
) -> VmResult<String> {
    match slot {
        Slot::Reference(None) => Ok("null".to_string()),
        Slot::Reference(Some(r)) => {
            let obj = heap.get(*r)?;
            match spec {
                's' => Ok(heap_object_to_string(obj, *r)),
                'd' => match obj.fields.first() {
                    Some(Slot::Int(v)) => Ok(v.to_string()),
                    Some(Slot::Long(v)) => Ok(v.to_string()),
                    _ => Ok("0".to_string()),
                },
                'f' => {
                    let v = match obj.fields.first() {
                        Some(Slot::Double(v)) => *v,
                        Some(Slot::Float(v)) => f64::from(*v),
                        _ => 0.0,
                    };
                    Ok(precision.map_or_else(|| format!("{v:.6}"), |p| format!("{v:.p$}")))
                }
                'x' => match obj.fields.first() {
                    Some(Slot::Int(v)) => Ok(format!("{v:x}")),
                    Some(Slot::Long(v)) => Ok(format!("{v:x}")),
                    _ => Ok("0".to_string()),
                },
                'X' => match obj.fields.first() {
                    Some(Slot::Int(v)) => Ok(format!("{v:X}")),
                    Some(Slot::Long(v)) => Ok(format!("{v:X}")),
                    _ => Ok("0".to_string()),
                },
                _ => Ok(String::new()),
            }
        }
        _ => Ok(String::new()),
    }
}

/// Native: `String.format(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/String;`
fn native_string_format(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let fmt_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let fmt = heap.get(fmt_ref)?.string_value.clone().unwrap_or_default();

    let arr_len = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.fields.len(),
        _ => 0,
    };

    let mut result = String::new();
    let mut arg_idx = 0usize;
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] != '%' {
            result.push(chars[i]);
            i += 1;
            continue;
        }
        i += 1;
        if i >= chars.len() {
            break;
        }

        // Parse optional precision: %.2f
        let precision: Option<usize> = if chars[i] == '.' {
            i += 1;
            let mut prec_str = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                prec_str.push(chars[i]);
                i += 1;
            }
            prec_str.parse().ok()
        } else {
            None
        };

        // Skip optional width digits
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }

        if i >= chars.len() {
            break;
        }
        let spec = chars[i];
        i += 1;

        match spec {
            '%' => result.push('%'),
            'n' => result.push('\n'),
            's' | 'd' | 'f' | 'x' | 'X' => {
                let slot = if arg_idx < arr_len {
                    match args.get(1) {
                        Some(Slot::Reference(Some(r))) => heap
                            .get(*r)?
                            .fields
                            .get(arg_idx)
                            .copied()
                            .unwrap_or(Slot::Reference(None)),
                        _ => Slot::Reference(None),
                    }
                } else {
                    Slot::Reference(None)
                };
                arg_idx += 1;
                let formatted = format_arg(spec, precision, &slot, heap)?;
                result.push_str(&formatted);
            }
            _ => {
                result.push('%');
                result.push(spec);
            }
        }
    }

    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

// ---- Extended String natives ----

/// Native: `String.toUpperCase()` — returns a new uppercase String.
fn native_string_touppercase(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.to_uppercase());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.toLowerCase()` — returns a new lowercase String.
fn native_string_tolowercase(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(s.to_lowercase());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.replace(char, char)` — replaces all occurrences of old char with new char.
#[allow(clippy::cast_sign_loss)]
fn native_string_replace_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let old_char = match args.get(1) {
        Some(Slot::Int(v)) => char::from_u32((*v).cast_unsigned()).unwrap_or('?'),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let new_char = match args.get(2) {
        Some(Slot::Int(v)) => char::from_u32((*v).cast_unsigned()).unwrap_or('?'),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let result = s.replace(old_char, &new_char.to_string());
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.replace(CharSequence, CharSequence)` — replaces all occurrences of target with replacement.
fn native_string_replace_charsequence(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let target_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let target = heap
        .get(target_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let replacement_ref = match args.get(2) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let replacement = heap
        .get(replacement_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let result = s.replace(&*target, &replacement);
    let r = heap.allocate_string(result);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `String.split(String)` — splits string by delimiter, returns String array.
fn native_string_split(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let delim_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let delim = heap
        .get(delim_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    let parts: Vec<&str> = s.split(&*delim).collect();
    let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), parts.len());
    for (i, part) in parts.iter().enumerate() {
        let str_ref = heap.allocate_string((*part).to_string());
        heap.get_mut(arr_ref).unwrap().fields[i] = Slot::Reference(Some(str_ref));
    }
    Ok(Some(Slot::Reference(Some(arr_ref))))
}

/// Native: `String.hashCode()` — Java's hash algorithm: `s[0]*31^(n-1) + s[1]*31^(n-2) + ... + s[n-1]`.
#[allow(clippy::cast_possible_wrap)]
fn native_string_hashcode(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let mut h: i32 = 0;
    for ch in s.chars() {
        h = h.wrapping_mul(31).wrapping_add(ch as i32);
    }
    Ok(Some(Slot::Int(h)))
}

/// Native: `String.toString()` — identity, returns `this`.
#[allow(clippy::unnecessary_wraps)]
fn native_string_tostring(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(Some(args.first().copied().unwrap_or(Slot::Reference(None))))
}

// ---- Math natives ----

/// Native: `Math.max(int, int)` — returns the larger value.
fn native_math_max_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Int(a.max(b))))
}

/// Native: `Math.min(int, int)` — returns the smaller value.
fn native_math_min_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let b = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Int(a.min(b))))
}

/// Native: `Math.abs(int)` — returns absolute value.
fn native_math_abs_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Int(a.wrapping_abs())))
}

// ---- Extended Math natives ----

/// Native: `Math.sqrt(double)` — returns square root.
fn native_math_sqrt(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Double(a.sqrt())))
}

/// Native: `Math.pow(double, double)` — returns a raised to the power b.
fn native_math_pow(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Double(a.powf(b))))
}

/// Native: `Math.floor(double)` — returns floor value.
fn native_math_floor(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Double(a.floor())))
}

/// Native: `Math.ceil(double)` — returns ceiling value.
fn native_math_ceil(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Double(a.ceil())))
}

/// Native: `Math.round(double)` — returns closest long.
#[allow(clippy::cast_possible_truncation)]
fn native_math_round_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Long(a.round() as i64)))
}

/// Native: `Math.abs(long)` — returns absolute value.
fn native_math_abs_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Long(a.wrapping_abs())))
}

/// Native: `Math.abs(double)` — returns absolute value.
fn native_math_abs_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Double(a.abs())))
}

/// Native: `Math.max(long, long)` — returns the larger value.
fn native_math_max_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Long(a.max(b))))
}

/// Native: `Math.min(long, long)` — returns the smaller value.
fn native_math_min_long(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Long(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let b = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Long(a.min(b))))
}

/// Native: `Math.max(double, double)` — returns the larger value.
fn native_math_max_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Double(a.max(b))))
}

/// Native: `Math.min(double, double)` — returns the smaller value.
fn native_math_min_double(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let a = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    let b = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    Ok(Some(Slot::Double(a.min(b))))
}

// ---- Long class natives ----

/// Native: `Long.parseLong(String)` — parses string to long.
fn native_long_parselong(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: i64 = s.trim().parse().map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(Some(Slot::Long(val)))
}

/// Native: `Long.valueOf(long)` — boxes long into Long object.
fn native_long_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate("java/lang/Long".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Long(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.longValue()` — unboxes Long to long.
fn native_long_longvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

/// Native: `Long.toString(long)` — static, converts long to String.
fn native_long_tostring_static(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Long(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let r = heap.allocate_string(val.to_string());
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Long.compareTo(Object)` — compares two boxed Longs.
fn native_long_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let long_val = |s: &Slot| -> VmResult<i64> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Long(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => long_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = match args.get(1) {
        Some(s) => long_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    Ok(Some(Slot::Int(ordering_to_int(a.cmp(&b)))))
}

// ---- Double class natives ----

/// Native: `Double.parseDouble(String)` — parses string to double.
fn native_double_parsedouble(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: f64 = s.trim().parse().map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(Some(Slot::Double(val)))
}

/// Native: `Double.valueOf(double)` — boxes double into Double object.
fn native_double_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    let r = heap.allocate("java/lang/Double".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Double(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Double.doubleValue()` — unboxes Double to double.
fn native_double_doublevalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

// ---- Float class native ----

/// Native: `Float.parseFloat(String)` — parses string to float.
fn native_float_parsefloat(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let str_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        Some(Slot::Reference(None)) => return Err(VmError::NullPointerException),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Reference",
                got: "other",
            });
        }
    };
    let s = heap.get(str_ref)?.string_value.clone().unwrap_or_default();
    let val: f32 = s.trim().parse().map_err(|_| VmError::JavaException {
        class_name: "java/lang/NumberFormatException".to_string(),
    })?;
    Ok(Some(Slot::Float(val)))
}

// ---- Boolean class native ----

/// Native: `Boolean.parseBoolean(String)` — case-insensitive "true" → 1, else 0.
fn native_boolean_parseboolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => {
            let s = heap.get(*r)?.string_value.clone().unwrap_or_default();
            let val = s.eq_ignore_ascii_case("true");
            Ok(Some(Slot::Int(i32::from(val))))
        }
        Some(Slot::Reference(None)) => Ok(Some(Slot::Int(0))),
        _ => Err(VmError::TypeMismatch {
            expected: "Reference",
            got: "other",
        }),
    }
}

/// Execute a `StringConcatFactory` recipe: walk the recipe string, replacing
/// `\u{1}` placeholders with stringified dynamic args from the operand stack.
pub(crate) fn execute_string_concat_recipe(
    recipe: &str,
    dynamic_args: &[Slot],
    arg_types: &[char],
    constants: &[String],
    heap: &mut duke_gc::Heap,
) -> VmResult<Slot> {
    let mut result = String::new();
    let mut dyn_idx = 0;
    let mut const_idx = 0;

    for ch in recipe.chars() {
        match ch {
            '\u{1}' => {
                if dyn_idx < dynamic_args.len() {
                    let type_hint = arg_types.get(dyn_idx).copied().unwrap_or('I');
                    stringify_slot(&dynamic_args[dyn_idx], type_hint, heap, &mut result)?;
                    dyn_idx += 1;
                }
            }
            '\u{2}' => {
                if const_idx < constants.len() {
                    result.push_str(&constants[const_idx]);
                    const_idx += 1;
                }
            }
            other => result.push(other),
        }
    }

    let r = heap.allocate_string(result);
    Ok(Slot::Reference(Some(r)))
}

/// Convert a Slot to its string representation (like Java's String.valueOf).
/// `type_hint` is the JVM type descriptor char: 'Z' for boolean, 'I' for int, etc.
fn stringify_slot(
    slot: &Slot,
    type_hint: char,
    heap: &duke_gc::Heap,
    out: &mut String,
) -> VmResult<()> {
    match slot {
        Slot::Int(v) => {
            if type_hint == 'Z' {
                // JVM boolean: 0 = false, nonzero = true.
                out.push_str(if *v != 0 { "true" } else { "false" });
            } else if type_hint == 'C' {
                // JVM char: render as the Unicode character.
                if let Some(ch) = char::from_u32((*v).cast_unsigned()) {
                    out.push(ch);
                } else {
                    out.push('?');
                }
            } else {
                out.push_str(&v.to_string());
            }
        }
        Slot::Long(v) => out.push_str(&v.to_string()),
        Slot::Float(v) => out.push_str(&format_java_float(*v)),
        Slot::Double(v) => out.push_str(&format_java_double(*v)),
        Slot::Reference(None) => out.push_str("null"),
        Slot::Reference(Some(r)) => {
            out.push_str(&heap_object_to_string(heap.get(*r)?, *r));
        }
        Slot::ReturnAddress(v) => out.push_str(&v.to_string()),
    }
    Ok(())
}

/// Format a float like Java's Float.toString.
fn format_java_float(v: f32) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    let s = format!("{v}");
    if s.contains('.') { s } else { format!("{v}.0") }
}

/// Format a double like Java's Double.toString.
fn format_java_double(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    let s = format!("{v}");
    if s.contains('.') { s } else { format!("{v}.0") }
}
fn native_sb_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get_mut(this_ref)?;
    obj.string_value = Some(String::new());
    Ok(None)
}

/// Native: `StringBuilder.<init>(Ljava/lang/String;)V` — init with string.
fn native_sb_init_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let init_str = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone().unwrap_or_default(),
        _ => String::new(),
    };
    let obj = heap.get_mut(this_ref)?;
    obj.string_value = Some(init_str);
    Ok(None)
}

/// Native: `StringBuilder.append(Ljava/lang/String;)Ljava/lang/StringBuilder;`
fn native_sb_append_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let append_str = match args.get(1) {
        Some(Slot::Reference(Some(r))) => heap.get(*r)?.string_value.clone().unwrap_or_default(),
        _ => "null".to_string(),
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&append_str);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(I)Ljava/lang/StringBuilder;`
fn native_sb_append_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(J)Ljava/lang/StringBuilder;`
fn native_sb_append_long(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = match args.get(1) {
        Some(Slot::Long(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Long",
                got: "other",
            });
        }
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(D)Ljava/lang/StringBuilder;`
fn native_sb_append_double(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = match args.get(1) {
        Some(Slot::Double(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Double",
                got: "other",
            });
        }
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(F)Ljava/lang/StringBuilder;`
fn native_sb_append_float(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = match args.get(1) {
        Some(Slot::Float(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Float",
                got: "other",
            });
        }
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&val.to_string());
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(Z)Ljava/lang/StringBuilder;`
fn native_sb_append_boolean(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = match args.get(1) {
        Some(Slot::Int(v)) => *v != 0,
        _ => false,
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(if val { "true" } else { "false" });
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.append(C)Ljava/lang/StringBuilder;`
fn native_sb_append_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = match args.get(1) {
        Some(Slot::Int(v)) => char::from_u32((*v).cast_unsigned()).unwrap_or('\0'),
        _ => '\0',
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push(val);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}

/// Native: `StringBuilder.toString()Ljava/lang/String;`
fn native_sb_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let content = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let r = heap.allocate_string(content);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `StringBuilder.length()I`
fn native_sb_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let len = heap
        .get(this_ref)?
        .string_value
        .as_ref()
        .map_or(0, String::len);
    Ok(Some(Slot::Int(i32::try_from(len).unwrap_or(i32::MAX))))
}

// Character natives
// ---------------------------------------------------------------------------

/// Helper: extract a `char` from a `Slot::Int` argument.
fn slot_to_char(slot: &Slot) -> VmResult<char> {
    match slot {
        Slot::Int(v) => Ok(char::from_u32((*v).cast_unsigned()).unwrap_or('\0')),
        _ => Err(VmError::TypeMismatch {
            expected: "Int (char)",
            got: "other",
        }),
    }
}

/// Native: `Character.isDigit(C)Z`
fn native_char_is_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_ascii_digit()))))
}

/// Native: `Character.isLetter(C)Z`
fn native_char_is_letter(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_alphabetic()))))
}

/// Native: `Character.isWhitespace(C)Z`
fn native_char_is_whitespace(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_whitespace()))))
}

/// Native: `Character.isUpperCase(C)Z`
fn native_char_is_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_uppercase()))))
}

/// Native: `Character.isLowerCase(C)Z`
fn native_char_is_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_lowercase()))))
}

/// Native: `Character.toUpperCase(C)C`
fn native_char_to_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    let upper = ch.to_uppercase().next().unwrap_or(ch);
    Ok(Some(Slot::Int(upper as i32)))
}

/// Native: `Character.toLowerCase(C)C`
fn native_char_to_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    let lower = ch.to_lowercase().next().unwrap_or(ch);
    Ok(Some(Slot::Int(lower as i32)))
}

/// Native: `Character.isLetterOrDigit(C)Z`
fn native_char_is_letter_or_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_alphanumeric()))))
}

/// Native: `Character.valueOf(C)Ljava/lang/Character;` — box a char.
fn native_char_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int (char)",
                got: "other",
            });
        }
    };
    let r = heap.allocate("java/lang/Character".to_string(), 1);
    heap.get_mut(r).unwrap().fields[0] = Slot::Int(val);
    Ok(Some(Slot::Reference(Some(r))))
}

/// Native: `Character.charValue()C` — unbox Character to char.
fn native_char_charvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = heap.get(this_ref)?.fields[0];
    Ok(Some(val))
}

// ---------------------------------------------------------------------------
// ArrayList natives
// ---------------------------------------------------------------------------

/// Native: `ArrayList.<init>()V` — initializes with size=0.
fn native_arraylist_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    heap.get_mut(this_ref)?.fields[0] = Slot::Int(0);
    Ok(None)
}

/// Native: `ArrayList.add(Object)Z` — appends element, returns true.
fn native_arraylist_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let element = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(VmError::NullPointerException), // shouldn't happen; init sets fields[0]=Int(0)
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1))) // boolean true
}

/// Native: `ArrayList.get(I)Object` — returns element at index.
#[allow(clippy::cast_sign_loss)]
fn native_arraylist_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let idx = match args.get(1) {
        Some(Slot::Int(i)) => *i as usize,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let obj = heap.get(this_ref)?;
    obj.fields.get(idx + 1).map_or_else(
        || {
            Err(VmError::JavaException {
                class_name: "java/lang/ArrayIndexOutOfBoundsException".to_string(),
            })
        },
        |slot| Ok(Some(*slot)),
    )
}

/// Native: `ArrayList.size()I`
fn native_arraylist_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get(this_ref)?;
    match obj.fields.first() {
        Some(Slot::Int(sz)) => Ok(Some(Slot::Int(*sz))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `ArrayList.iterator()Iterator` — creates an `ArrayListIterator`.
fn native_arraylist_iterator(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let iter_ref = heap.allocate("duke/util/ArrayListIterator".to_string(), 2);
    {
        let iter_obj = heap.get_mut(iter_ref)?;
        iter_obj.fields[0] = Slot::Reference(Some(this_ref));
        iter_obj.fields[1] = Slot::Int(0);
    }
    Ok(Some(Slot::Reference(Some(iter_ref))))
}

/// Native: `ArrayList.sort(Comparator)V` — sorts in-place using insertion sort,
/// calling `compareTo` on each element pair via the interpreter callback.
///
/// Only null Comparator (natural ordering via `compareTo`) is supported.
fn array_list_sort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    // args[0] = ArrayList ref, args[1] = Comparator (null = natural ordering)
    let list_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };

    // Fix 1: use the correct error variant for unsupported non-null Comparator.
    if !matches!(args.get(1), Some(Slot::Reference(None)) | None) {
        return Err(VmError::Unimplemented {
            mnemonic: "ArrayList.sort(non-null Comparator)",
        });
    }

    // Fix 2: guard against a negative size stored in fields[0].
    let size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(VmError::NegativeArraySize { size: *n }),
        _ => return Ok(None),
    };

    if size <= 1 {
        return Ok(None);
    }

    // Collect element refs (fields[1..=size]).
    let mut elems: Vec<u64> = (1..=size)
        .filter_map(|i| match heap.get(list_ref).ok()?.fields.get(i) {
            Some(Slot::Reference(Some(r))) => Some(*r),
            _ => None,
        })
        .collect();

    // Fix 3: malformed list → InvalidRef, not silent Ok(None).
    if elems.len() != size {
        return Err(VmError::InvalidRef { address: list_ref });
    }

    // Insertion sort — O(n²), correct, easy to verify.
    for i in 1..elems.len() {
        let mut key = elems[i];
        let mut j = i;
        while j > 0 {
            let receiver = elems[j - 1];
            let class_name = heap.get(receiver)?.class_name.clone();
            let cmp = ops.invoke(
                heap,
                output,
                &class_name,
                COMPARE_TO_METHOD,
                COMPARE_TO_OBJECT_DESC,
                vec![Slot::Reference(Some(receiver)), Slot::Reference(Some(key))],
            )?;

            // Patch stale young-gen refs if a minor GC fired during the callback.
            // Guarded by `has_pending_forwards` so the common (no-GC) path pays
            // only one bool check instead of O(n) HashMap probes.
            if heap.has_pending_forwards() {
                for elem in &mut elems {
                    let mut slot = Slot::Reference(Some(*elem));
                    heap.apply_forward(&mut slot);
                    if let Slot::Reference(Some(r)) = slot {
                        *elem = r;
                    }
                }
                // Re-read key after forwarding patch (it lives outside elems
                // during the innermost loop iteration).
                let mut key_slot = Slot::Reference(Some(key));
                heap.apply_forward(&mut key_slot);
                if let Slot::Reference(Some(r)) = key_slot {
                    key = r;
                }
            }

            // Fix 4: explicit error on non-Int compareTo return.
            match cmp {
                Some(Slot::Int(n)) if n <= 0 => break,
                Some(Slot::Int(_)) => {} // n > 0, keep shifting
                _ => {
                    return Err(VmError::TypeMismatch {
                        expected: "Int",
                        got: "other",
                    });
                }
            }
            elems[j] = elems[j - 1];
            j -= 1;
        }
        elems[j] = key;
    }

    // Write sorted elements back using the write barrier.
    for (i, &r) in elems.iter().enumerate() {
        heap.write_field(list_ref, i + 1, Slot::Reference(Some(r)))?;
    }

    Ok(None)
}

// ---------------------------------------------------------------------------
// Collections natives
// ---------------------------------------------------------------------------

/// Native: `Collections.sort(List)V` — delegates to the list's sort(null) method.
///
/// `Collections.sort(list)` is compiled by javac as
/// `invokestatic java/util/Collections.sort:(Ljava/util/List;)V`.
/// We forward to the runtime class's `sort(Comparator=null)`, which for an
/// `ArrayList` performs the insertion-sort-with-compareTo callback.
fn native_collections_sort(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> VmResult<Option<Slot>> {
    // args[0] = List ref
    let list_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    // Dispatch on the actual runtime class so any List implementation works.
    let class_name = heap.get(list_ref)?.class_name.clone();
    ops.invoke(
        heap,
        output,
        &class_name,
        "sort",
        SORT_COMPARATOR_DESC,
        vec![Slot::Reference(Some(list_ref)), Slot::Reference(None)],
    )?;
    Ok(None)
}

// ---------------------------------------------------------------------------
// ArrayListIterator natives
// ---------------------------------------------------------------------------

/// Native: `ArrayListIterator.<init>` — no-op; fields set directly by `native_arraylist_iterator`.
#[allow(clippy::unnecessary_wraps)]
fn native_arraylist_iter_init(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    Ok(None)
}

/// Native: `ArrayListIterator.hasNext()Z`
fn native_arraylist_iter_hasnext(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let iter_obj = heap.get(this_ref)?;
    let list_ref = match iter_obj.fields.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Ok(Some(Slot::Int(0))),
    };
    let cursor = match iter_obj.fields.get(1) {
        Some(Slot::Int(i)) => *i,
        _ => 0,
    };
    let list_size = match heap.get(list_ref)?.fields.first() {
        Some(Slot::Int(sz)) => *sz,
        _ => 0,
    };
    Ok(Some(Slot::Int(i32::from(cursor < list_size))))
}

/// Native: `ArrayListIterator.next()Object` — returns element at cursor, advances cursor.
#[allow(clippy::cast_sign_loss)]
fn native_arraylist_iter_next(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let (list_ref, cursor) = {
        let iter_obj = heap.get(this_ref)?;
        let lr = match iter_obj.fields.first() {
            Some(Slot::Reference(Some(r))) => *r,
            _ => return Err(VmError::NullPointerException),
        };
        let c = match iter_obj.fields.get(1) {
            Some(Slot::Int(i)) => *i,
            _ => 0,
        };
        (lr, c)
    };
    let element = {
        let list_obj = heap.get(list_ref)?;
        match list_obj.fields.get(cursor as usize + 1) {
            Some(slot) => *slot,
            None => {
                return Err(VmError::JavaException {
                    class_name: "java/util/NoSuchElementException".to_string(),
                });
            }
        }
    };
    heap.get_mut(this_ref)?.fields[1] = Slot::Int(cursor + 1);
    Ok(Some(element))
}

// ---- Double.isNaN ----

/// Native: `Double.isNaN(D)Z` — returns 1 if value is NaN.
#[allow(clippy::unnecessary_wraps)]
fn native_double_isnan(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Double(v)) => Ok(Some(Slot::Int(i32::from(v.is_nan())))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `Double.compareTo(Object)` — compares two boxed Doubles.
fn native_double_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let double_val = |s: &Slot| -> VmResult<f64> {
        match s {
            Slot::Reference(Some(r)) => match heap.get(*r)?.fields.first() {
                Some(Slot::Double(n)) => Ok(*n),
                _ => Err(VmError::InvalidRef { address: *r }),
            },
            _ => Err(VmError::NullPointerException),
        }
    };
    let a = match args.first() {
        Some(s) => double_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    let b = match args.get(1) {
        Some(s) => double_val(s)?,
        None => return Err(VmError::NullPointerException),
    };
    // Use total_cmp: implements Java's total order where NaN > +∞ > … > -∞.
    Ok(Some(Slot::Int(ordering_to_int(a.total_cmp(&b)))))
}

// ---- Arrays natives ----

/// Native: `Arrays.fill(int[], int)` — fills all elements with val.
fn native_arrays_fill_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = match args.get(1) {
        Some(Slot::Int(v)) => Slot::Int(*v),
        _ => Slot::Int(0),
    };
    let obj = heap.get_mut(arr_ref)?;
    for slot in &mut obj.fields {
        *slot = val;
    }
    Ok(None)
}

/// Native: `Arrays.fill(Object[], Object)` — fills all elements with val.
fn native_arrays_fill_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let obj = heap.get_mut(arr_ref)?;
    for slot in &mut obj.fields {
        *slot = val;
    }
    Ok(None)
}

/// Native: `Arrays.copyOf(int[], int)` — copies to new int[] of given length.
fn native_arrays_copyof_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let new_len = match args.get(1) {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(VmError::NegativeArraySize { size: *n }),
        _ => 0,
    };
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[I".to_string(), new_len);
    let dst = heap.get_mut(dst_ref)?;
    for i in 0..new_len {
        dst.fields[i] = src_fields.get(i).copied().unwrap_or(Slot::Int(0));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Arrays.copyOf(Object[], int)` — copies to new Object[] of given length.
fn native_arrays_copyof_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let src_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let new_len = match args.get(1) {
        Some(Slot::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
        Some(Slot::Int(n)) => return Err(VmError::NegativeArraySize { size: *n }),
        _ => 0,
    };
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[Ljava/lang/Object;".to_string(), new_len);
    let dst = heap.get_mut(dst_ref)?;
    for i in 0..new_len {
        dst.fields[i] = src_fields.get(i).copied().unwrap_or(Slot::Reference(None));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}

/// Native: `Arrays.sort(int[])` — sorts fields in place.
fn native_arrays_sort_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let arr_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get_mut(arr_ref)?;
    obj.fields.sort_by(|a, b| match (a, b) {
        (Slot::Int(x), Slot::Int(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    });
    Ok(None)
}

// ---------------------------------------------------------------------------
// HashMap natives
// ---------------------------------------------------------------------------

/// Semantic equality for `HashMap` keys: compares by `string_value` for heap strings,
/// or by the first field (e.g. intValue) for boxed numerics, or by reference identity.
fn slots_equal(a: &Slot, b: &Slot, heap: &duke_gc::Heap) -> bool {
    match (a, b) {
        (Slot::Reference(None), Slot::Reference(None)) => true,
        (Slot::Reference(Some(ra)), Slot::Reference(Some(rb))) => {
            if ra == rb {
                return true;
            }
            let Ok(oa) = heap.get(*ra) else { return false };
            let Ok(ob) = heap.get(*rb) else { return false };
            if oa.class_name == "java/lang/String" || ob.class_name == "java/lang/String" {
                return oa.string_value == ob.string_value;
            }
            if oa.class_name == ob.class_name {
                oa.fields.first() == ob.fields.first()
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Native: `HashMap.<init>()V` — initialises size counter at fields\[0\] to 0.
fn find_hashmap_entry_index(fields: &[Slot], key: &Slot, heap: &duke_gc::Heap) -> Option<usize> {
    (1..fields.len())
        .step_by(2)
        .find(|&i| i + 1 < fields.len() && slots_equal(&fields[i], key, heap))
}

fn native_hashmap_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.is_empty() {
        obj.fields.push(Slot::Int(0));
    } else {
        obj.fields[0] = Slot::Int(0);
    }
    Ok(None)
}

/// Native: `HashMap.put(Object, Object)Object` — inserts or updates a key-value pair.
/// Returns the old value if the key was already present, or null if it is new.
fn native_hashmap_put(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let val = args.get(2).copied().unwrap_or(Slot::Reference(None));
    // Clone fields to release the immutable borrow before mutating.
    let fields = heap.get(this_ref)?.fields.clone();

    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        let old = fields[i + 1];
        heap.get_mut(this_ref)?.fields[i + 1] = val;
        return Ok(Some(old));
    }

    // New key — append pair and bump size.
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(VmError::NullPointerException),
    }
    obj.fields.push(key);
    obj.fields.push(val);
    Ok(Some(Slot::Reference(None)))
}

/// Native: `HashMap.get(Object)Object` — returns value for key, or null if absent.
fn native_hashmap_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();

    Ok(Some(
        find_hashmap_entry_index(&fields, &key, heap)
            .map_or(Slot::Reference(None), |i| fields[i + 1]),
    ))
}

/// Native: `HashMap.containsKey(Object)Z` — returns 1 if key present, 0 otherwise.
fn native_hashmap_contains_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();

    if find_hashmap_entry_index(&fields, &key, heap).is_some() {
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `HashMap.size()I` — returns entry count from fields\[0\].
fn native_hashmap_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Ok(Some(Slot::Int(*n))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `HashMap.remove(Object)Object` — removes a key-value pair, returns old value or null.
/// Uses swap-remove (swaps target pair with last pair) for O(1) deletion.
fn native_hashmap_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();

    if let Some(i) = find_hashmap_entry_index(&fields, &key, heap) {
        let old_val = fields[i + 1];
        let obj = heap.get_mut(this_ref)?;
        let last_val_idx = obj.fields.len() - 1;
        let last_key_idx = obj.fields.len() - 2;
        obj.fields.swap(i + 1, last_val_idx);
        obj.fields.swap(i, last_key_idx);
        obj.fields.truncate(obj.fields.len() - 2);
        match obj.fields.first_mut() {
            Some(Slot::Int(sz)) => *sz -= 1,
            _ => return Err(VmError::NullPointerException),
        }
        Ok(Some(old_val))
    } else {
        Ok(Some(Slot::Reference(None)))
    }
}

/// Native: `HashMap.isEmpty()Z` — returns 1 if size == 0, else 0.
fn native_hashmap_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) => Ok(Some(Slot::Int(1))),
        Some(Slot::Int(_)) => Ok(Some(Slot::Int(0))),
        _ => Ok(Some(Slot::Int(1))),
    }
}

/// Native: `HashMap.getOrDefault(Object, Object)Object` — returns value for key, or default if absent.
fn native_hashmap_get_or_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let key = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let default = args.get(2).copied().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();

    Ok(Some(
        find_hashmap_entry_index(&fields, &key, heap).map_or(default, |i| fields[i + 1]),
    ))
}

// ---------------------------------------------------------------------------
// HashSet natives
// ---------------------------------------------------------------------------

/// Native: `HashSet.<init>()V` — initialises size counter at fields\[0\] to 0.
fn find_hashset_entry_index(
    fields: &[Slot],
    element: &Slot,
    heap: &duke_gc::Heap,
) -> Option<usize> {
    fields
        .iter()
        .skip(1)
        .position(|field| slots_equal(field, element, heap))
        .map(|idx| idx + 1)
}

fn native_hashset_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let obj = heap.get_mut(this_ref)?;
    if obj.fields.is_empty() {
        obj.fields.push(Slot::Int(0));
    } else {
        obj.fields[0] = Slot::Int(0);
    }
    Ok(None)
}

/// Native: `HashSet.add(Object)Z` — adds element if not already present.
/// Returns 1 if added, 0 if element was already in the set.
fn native_hashset_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let element = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();
    // fields[0] = size, fields[1..] = elements
    if find_hashset_entry_index(&fields, &element, heap).is_some() {
        return Ok(Some(Slot::Int(0))); // duplicate
    }

    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(VmError::NullPointerException),
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1)))
}

/// Native: `HashSet.contains(Object)Z` — returns 1 if element is present, 0 otherwise.
fn native_hashset_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let element = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();

    if find_hashset_entry_index(&fields, &element, heap).is_some() {
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `HashSet.remove(Object)Z` — removes element if present, returns 1 if removed, 0 if absent.
/// Uses swap-remove (swaps target with last element) for O(1) deletion.
fn native_hashset_remove(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let element = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();

    if let Some(i) = find_hashset_entry_index(&fields, &element, heap) {
        let obj = heap.get_mut(this_ref)?;
        let last_idx = obj.fields.len() - 1;
        obj.fields.swap(i, last_idx);
        obj.fields.truncate(obj.fields.len() - 1);
        match obj.fields.first_mut() {
            Some(Slot::Int(sz)) => *sz -= 1,
            _ => return Err(VmError::NullPointerException),
        }
        Ok(Some(Slot::Int(1)))
    } else {
        Ok(Some(Slot::Int(0)))
    }
}

/// Native: `HashSet.size()I` — returns element count from fields\[0\].
fn native_hashset_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(n)) => Ok(Some(Slot::Int(*n))),
        _ => Ok(Some(Slot::Int(0))),
    }
}

/// Native: `HashSet.isEmpty()Z` — returns 1 if size == 0, else 0.
fn native_hashset_is_empty(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    match heap.get(this_ref)?.fields.first() {
        Some(Slot::Int(0)) => Ok(Some(Slot::Int(1))),
        Some(Slot::Int(_)) => Ok(Some(Slot::Int(0))),
        _ => Ok(Some(Slot::Int(1))),
    }
}

const PROCESS_ID_FIELD: usize = 0;
const PROCESS_STDIN_FIELD: usize = 1;
const PROCESS_STDOUT_FIELD: usize = 2;
const PROCESS_STDERR_FIELD: usize = 3;

fn string_array_from_slot(slot: Slot, heap: &duke_gc::Heap) -> VmResult<Vec<String>> {
    let Slot::Reference(Some(array_ref)) = slot else {
        return Err(VmError::NullPointerException);
    };
    let elements = heap.get(array_ref)?.fields.clone();
    elements
        .into_iter()
        .map(|element| match element {
            Slot::Reference(Some(string_ref)) => string_value_from_ref(heap, string_ref),
            _ => Err(VmError::NullPointerException),
        })
        .collect()
}

fn optional_file_path_from_slot(
    slot: Slot,
    heap: &duke_gc::Heap,
) -> VmResult<Option<std::path::PathBuf>> {
    match slot {
        Slot::Reference(Some(file_ref)) => Ok(Some(file_path_from_ref(file_ref, heap)?)),
        Slot::Reference(None) => Ok(None),
        _ => Err(VmError::NullPointerException),
    }
}

fn allocate_process_impl(
    heap: &mut duke_gc::Heap,
    ids: duke_gc::SpawnedProcessIds,
) -> VmResult<Option<Slot>> {
    let process_ref = heap.allocate("java/lang/ProcessImpl".to_string(), 4);
    let process_obj = heap.get_mut(process_ref)?;
    process_obj.fields[PROCESS_ID_FIELD] = Slot::Int(ids.process_id);
    process_obj.fields[PROCESS_STDIN_FIELD] = Slot::Int(ids.stdin_id);
    process_obj.fields[PROCESS_STDOUT_FIELD] = Slot::Int(ids.stdout_id);
    process_obj.fields[PROCESS_STDERR_FIELD] = Slot::Int(ids.stderr_id);
    Ok(Some(Slot::Reference(Some(process_ref))))
}

fn spawn_process_impl(
    heap: &mut duke_gc::Heap,
    command: &[String],
    cwd: Option<&std::path::Path>,
) -> VmResult<Option<Slot>> {
    let ids = heap.spawn_host_process(command, cwd)?;
    allocate_process_impl(heap, ids)
}

fn process_field_id_from_this(
    args: &[Slot],
    heap: &duke_gc::Heap,
    field_idx: usize,
) -> VmResult<i32> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    match heap.get(this_ref)?.fields.get(field_idx) {
        Some(Slot::Int(id)) if *id > 0 => Ok(*id),
        _ => Err(VmError::JavaException {
            class_name: "java/io/IOException".into(),
        }),
    }
}

fn allocate_process_stream(
    heap: &mut duke_gc::Heap,
    class_name: &str,
    handle_id: i32,
) -> VmResult<Option<Slot>> {
    let stream_ref = heap.allocate(class_name.to_string(), 1);
    heap.get_mut(stream_ref)?.fields[0] = Slot::Int(handle_id);
    Ok(Some(Slot::Reference(Some(stream_ref))))
}

fn native_process_builder_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let command_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let builder_obj = heap.get_mut(this_ref)?;
    if builder_obj.fields.len() < 2 {
        return Err(VmError::InvalidRef { address: this_ref });
    }
    builder_obj.fields[0] = command_slot;
    builder_obj.fields[1] = Slot::Reference(None);
    Ok(None)
}

fn native_process_builder_directory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let directory_slot = args.get(1).copied().unwrap_or(Slot::Reference(None));
    let builder_obj = heap.get_mut(this_ref)?;
    if builder_obj.fields.len() < 2 {
        return Err(VmError::InvalidRef { address: this_ref });
    }
    builder_obj.fields[1] = directory_slot;
    Ok(Some(Slot::Reference(Some(this_ref))))
}

fn native_process_builder_start(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let builder_obj = heap.get(this_ref)?;
    let command_slot = builder_obj
        .fields
        .first()
        .copied()
        .unwrap_or(Slot::Reference(None));
    let directory_slot = builder_obj
        .fields
        .get(1)
        .copied()
        .unwrap_or(Slot::Reference(None));
    let command = string_array_from_slot(command_slot, heap)?;
    let cwd = optional_file_path_from_slot(directory_slot, heap)?;
    spawn_process_impl(heap, &command, cwd.as_deref())
}

#[allow(clippy::unnecessary_wraps)] // must match NativeHandler signature
fn native_runtime_get_runtime(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let runtime_ref = heap.allocate("java/lang/Runtime".to_string(), 0);
    Ok(Some(Slot::Reference(Some(runtime_ref))))
}

fn native_runtime_exec_array(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(_))) => {}
        _ => return Err(VmError::NullPointerException),
    }
    let command =
        string_array_from_slot(args.get(1).copied().unwrap_or(Slot::Reference(None)), heap)?;
    spawn_process_impl(heap, &command, None)
}

fn native_runtime_exec_array_dir(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(_))) => {}
        _ => return Err(VmError::NullPointerException),
    }
    let command =
        string_array_from_slot(args.get(1).copied().unwrap_or(Slot::Reference(None)), heap)?;
    let cwd =
        optional_file_path_from_slot(args.get(3).copied().unwrap_or(Slot::Reference(None)), heap)?;
    spawn_process_impl(heap, &command, cwd.as_deref())
}

fn native_process_get_input_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let stdout_id = process_field_id_from_this(args, heap, PROCESS_STDOUT_FIELD)?;
    allocate_process_stream(heap, "duke/process/ProcessInputStream", stdout_id)
}

fn native_process_get_error_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let stderr_id = process_field_id_from_this(args, heap, PROCESS_STDERR_FIELD)?;
    allocate_process_stream(heap, "duke/process/ProcessErrorStream", stderr_id)
}

fn native_process_get_output_stream(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let stdin_id = process_field_id_from_this(args, heap, PROCESS_STDIN_FIELD)?;
    allocate_process_stream(heap, "duke/process/ProcessOutputStream", stdin_id)
}

fn native_process_wait_for(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    Ok(Some(Slot::Int(heap.wait_host_process(process_id)?)))
}

fn native_process_exit_value(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    let Some(exit_code) = heap.try_host_process_exit_value(process_id)? else {
        return Err(VmError::JavaException {
            class_name: "java/lang/IllegalThreadStateException".into(),
        });
    };
    Ok(Some(Slot::Int(exit_code)))
}

fn native_process_destroy(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> VmResult<Option<Slot>> {
    let process_id = process_field_id_from_this(args, heap, PROCESS_ID_FIELD)?;
    heap.destroy_host_process(process_id)?;
    Ok(None)
}

// ---------------------------------------------------------------------------
// GC root gathering
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use duke_bytecode::Instruction;
    use duke_bytecode::instruction::ArrayType;
    use duke_classfile::types::CpEntry;
    use duke_loader::ClassLoader;
    use duke_runtime::{Slot, VmError, VmResult};

    macro_rules! wrap_simple_native_for_tests {
        ($($name:ident),* $(,)?) => {
            $(
                fn $name(
                    args: &[Slot],
                    heap: &mut duke_gc::Heap,
                    out: &mut dyn std::io::Write,
                ) -> VmResult<Option<Slot>> {
                    super::$name(args, heap, out, &mut NativeControl::default())
                }
            )*
        };
    }

    wrap_simple_native_for_tests!(
        native_arraylist_get,
        native_arrays_copyof_int,
        native_arrays_copyof_object,
        native_arrays_fill_object,
        native_boolean_parseboolean,
        native_double_doublevalue,
        native_double_parsedouble,
        native_float_parsefloat,
        native_hashmap_contains_key,
        native_hashmap_get,
        native_hashmap_get_or_default,
        native_hashmap_init,
        native_hashmap_put,
        native_hashmap_remove,
        native_hashmap_size,
        native_integer_parseint,
        native_long_parselong,
        native_math_min_double,
        native_object_tostring,
        native_print_boolean,
        native_print_char,
        native_print_double,
        native_print_float,
        native_print_long,
        native_print_object,
        native_print_string,
        native_println_boolean,
        native_println_object,
        native_println_string,
        native_sb_append_char,
        native_sb_append_string,
        native_sb_init_string,
        native_string_concat,
        native_string_contains,
        native_string_equals,
        native_string_format,
        native_string_indexof,
        native_string_replace_charsequence,
        native_string_split,
        native_string_substring,
        native_string_substring_range,
        native_string_value_of_object,
        native_system_exit,
    );

    fn array_list_sort(
        args: &[Slot],
        heap: &mut duke_gc::Heap,
        out: &mut dyn std::io::Write,
        ops: &mut dyn CallbackOps,
    ) -> VmResult<Option<Slot>> {
        super::array_list_sort(args, heap, out, &mut NativeControl::default(), ops)
    }

    struct NoopCallbackOps;

    impl CallbackOps for NoopCallbackOps {
        fn invoke(
            &mut self,
            _heap: &mut duke_gc::Heap,
            _output: &mut dyn Write,
            _class: &str,
            _method: &str,
            _descriptor: &str,
            _args: Vec<Slot>,
        ) -> VmResult<Option<Slot>> {
            Ok(None)
        }

        fn ensure_loaded(&mut self, _class: &str) -> VmResult<()> {
            Ok(())
        }

        fn inspect_class(&mut self, _class: &str) -> VmResult<ReflectedClassInfo> {
            Ok(ReflectedClassInfo {
                internal_name: String::new(),
                binary_name: String::new(),
                methods: Vec::new(),
                fields: Vec::new(),
            })
        }
    }

    // ---- Unit tests: hand-crafted instruction streams ----

    #[test]
    fn class_registry_all_classes_iterates_registered() {
        let reg = ClassRegistry::new();
        // registry starts empty; verify iteration works
        let count = reg.all_classes().count();
        assert_eq!(count, 0); // before bootstrap
    }

    #[test]
    fn native_registry_register_callback_can_be_looked_up() {
        #[allow(clippy::unnecessary_wraps)]
        fn dummy_cb(
            _args: &[Slot],
            _heap: &mut duke_gc::Heap,
            _out: &mut dyn std::io::Write,
            _control: &mut NativeControl,
            _ops: &mut dyn CallbackOps,
        ) -> VmResult<Option<Slot>> {
            Ok(None)
        }
        let mut reg = NativeRegistry::new();
        reg.register_callback("Test", "method", "()V", dummy_cb);
        assert!(matches!(
            reg.get_kind("Test", "method", "()V"),
            Some(HandlerKind::Callback(_))
        ));
    }

    #[test]
    fn native_registry_register_simple_stays_simple() {
        #[allow(clippy::unnecessary_wraps)]
        fn dummy(
            _args: &[Slot],
            _heap: &mut duke_gc::Heap,
            _out: &mut dyn std::io::Write,
            _control: &mut NativeControl,
        ) -> VmResult<Option<Slot>> {
            Ok(None)
        }
        let mut reg = NativeRegistry::new();
        reg.register("Test", "method", "()V", dummy);
        assert!(matches!(
            reg.get_kind("Test", "method", "()V"),
            Some(HandlerKind::Simple(_))
        ));
    }

    #[test]
    fn execute_iconst_ireturn() {
        let instructions = vec![(0, Instruction::Iconst1), (1, Instruction::Ireturn)];
        let result = execute(&instructions, &[], vec![], 2, 1).expect("should execute");
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn execute_bipush_istore_iload() {
        // bipush 42, istore_0, iload_0, ireturn
        let instructions = vec![
            (0, Instruction::Bipush(42)),
            (2, Instruction::Istore0),
            (3, Instruction::Iload0),
            (4, Instruction::Ireturn),
        ];
        let result = execute(&instructions, &[], vec![], 2, 2).unwrap();
        assert_eq!(result, Some(Slot::Int(42)));
    }

    #[test]
    fn execute_loads_args() {
        // iload_0, iload_1, pop, ireturn — returns first arg
        let instructions = vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::Pop),
            (3, Instruction::Ireturn),
        ];
        let result = execute(&instructions, &[], vec![Slot::Int(99), Slot::Int(0)], 2, 2).unwrap();
        assert_eq!(result, Some(Slot::Int(99)));
    }

    #[test]
    fn execute_sipush() {
        let instructions = vec![(0, Instruction::Sipush(1000)), (3, Instruction::Ireturn)];
        let result = execute(&instructions, &[], vec![], 2, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(1000)));
    }

    #[test]
    fn hand_coded_add() {
        // iload_0, iload_1, iadd, ireturn
        let instrs = vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::Iadd),
            (3, Instruction::Ireturn),
        ];
        let result = execute(&instrs, &[], vec![Slot::Int(3), Slot::Int(4)], 2, 2).unwrap();
        assert_eq!(result, Some(Slot::Int(7)));
    }

    #[test]
    fn hand_coded_div_by_zero() {
        let instrs = vec![
            (0, Instruction::Iload0),
            (1, Instruction::Iload1),
            (2, Instruction::Idiv),
            (3, Instruction::Ireturn),
        ];
        let err = execute(&instrs, &[], vec![Slot::Int(10), Slot::Int(0)], 2, 2).unwrap_err();
        assert!(matches!(err, VmError::DivisionByZero));
    }

    #[test]
    fn hand_coded_long_add() {
        let instrs = vec![
            (0, Instruction::Lload0),
            (1, Instruction::Lload1),
            (2, Instruction::Ladd),
            (3, Instruction::Lreturn),
        ];
        let result = execute(
            &instrs,
            &[],
            vec![Slot::Long(1_000_000_000), Slot::Long(2_000_000_000)],
            2,
            2,
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Long(3_000_000_000)));
    }

    #[test]
    fn hand_coded_ifeq_taken() {
        // iconst_0 (pc=0), ifeq +5 (pc=1, target=6), iconst_1 (pc=4), ireturn (pc=5),
        // iconst_2 (pc=6), ireturn (pc=7)
        let instrs = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Ifeq(5)), // 0 == 0, jump to pc=6
            (4, Instruction::Iconst1),
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst2),
            (7, Instruction::Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    #[test]
    fn hand_coded_ifeq_not_taken() {
        let instrs = vec![
            (0, Instruction::Iconst1), // push 1
            (1, Instruction::Ifeq(5)), // 1 != 0, NOT taken
            (4, Instruction::Iconst1), // reached
            (5, Instruction::Ireturn),
            (6, Instruction::Iconst2),
            (7, Instruction::Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn hand_coded_goto() {
        // iconst_5 (pc=0), goto +4 (pc=1, target=5), pop (pc=4, skipped),
        // ireturn (pc=5) — returns 5
        let instrs = vec![
            (0, Instruction::Iconst5),
            (1, Instruction::Goto(4)), // jump to pc=5
            (4, Instruction::Pop),     // skipped
            (5, Instruction::Ireturn), // returns 5
        ];
        let result = execute(&instrs, &[], vec![], 2, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(5)));
    }

    // ---- Integration tests: load Arithmetic.class and execute real bytecode ----

    fn fixture(name: &str) -> std::path::PathBuf {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../tests/fixtures");
        p.push(name);
        p
    }

    /// Parse and execute a static method from a `.class` file that takes `i32` args
    /// and returns an `i32`.
    fn run_static_int(class_name: &str, method_name: &str, args: Vec<i32>) -> i32 {
        use duke_bytecode::decode;
        use duke_classfile::{
            parse,
            types::{AttributeData, CpEntry},
        };

        let bytes = std::fs::read(fixture(class_name)).expect("fixture not found");
        let cf = parse(&bytes).expect("parse failed");

        let method = cf
            .methods
            .iter()
            .find(|m| {
                let Some(Some(CpEntry::Utf8(s))) = cf.constant_pool.get(m.name_index.0 as usize)
                else {
                    return false;
                };
                s.as_str() == method_name
            })
            .unwrap_or_else(|| panic!("method '{method_name}' not found"));

        let code = method
            .attributes
            .iter()
            .find_map(|a| {
                if let AttributeData::Code(c) = &a.data {
                    Some(c)
                } else {
                    None
                }
            })
            .expect("no Code attribute");

        let instructions = decode(&code.code).expect("decode failed");
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();

        match execute(
            &instructions,
            &cf.constant_pool,
            slots,
            code.max_stack,
            code.max_locals,
        )
        .expect("execute failed")
        {
            Some(Slot::Int(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    // Basic arithmetic
    #[test]
    fn int_add() {
        assert_eq!(run_static_int("Arithmetic.class", "add", vec![3, 4]), 7);
    }
    #[test]
    fn int_subtract() {
        assert_eq!(
            run_static_int("Arithmetic.class", "subtract", vec![10, 3]),
            7
        );
    }
    #[test]
    fn int_multiply() {
        assert_eq!(
            run_static_int("Arithmetic.class", "multiply", vec![3, 4]),
            12
        );
    }
    #[test]
    fn int_divide() {
        assert_eq!(run_static_int("Arithmetic.class", "divide", vec![10, 2]), 5);
    }
    #[test]
    fn int_remainder() {
        assert_eq!(
            run_static_int("Arithmetic.class", "remainder", vec![10, 3]),
            1
        );
    }
    #[test]
    fn int_negate() {
        assert_eq!(run_static_int("Arithmetic.class", "negate", vec![-5]), 5);
    }
    #[test]
    fn int_shift_left() {
        assert_eq!(
            run_static_int("Arithmetic.class", "shiftLeft", vec![1, 4]),
            16
        );
    }
    #[test]
    fn int_bitwise_and() {
        assert_eq!(
            run_static_int("Arithmetic.class", "bitwiseAnd", vec![0b1111, 0b1010]),
            0b1010
        );
    }
    #[test]
    fn int_bitwise_or() {
        assert_eq!(
            run_static_int("Arithmetic.class", "bitwiseOr", vec![0b1111, 0b1010]),
            0b1111
        );
    }

    // Conditionals
    #[test]
    fn int_max_a_wins() {
        assert_eq!(run_static_int("Arithmetic.class", "max", vec![7, 3]), 7);
    }
    #[test]
    fn int_max_b_wins() {
        assert_eq!(run_static_int("Arithmetic.class", "max", vec![3, 7]), 7);
    }
    #[test]
    fn int_abs_neg() {
        assert_eq!(run_static_int("Arithmetic.class", "abs", vec![-5]), 5);
    }
    #[test]
    fn int_abs_pos() {
        assert_eq!(run_static_int("Arithmetic.class", "abs", vec![5]), 5);
    }
    #[test]
    fn int_clamp_mid() {
        assert_eq!(
            run_static_int("Arithmetic.class", "clamp", vec![5, 1, 10]),
            5
        );
    }
    #[test]
    fn int_clamp_lo() {
        assert_eq!(
            run_static_int("Arithmetic.class", "clamp", vec![0, 1, 10]),
            1
        );
    }
    #[test]
    fn int_clamp_hi() {
        assert_eq!(
            run_static_int("Arithmetic.class", "clamp", vec![15, 1, 10]),
            10
        );
    }

    // Control flow / loops
    #[test]
    fn int_factorial_0() {
        assert_eq!(run_static_int("Arithmetic.class", "factorial", vec![0]), 1);
    }
    #[test]
    fn int_factorial_5() {
        assert_eq!(
            run_static_int("Arithmetic.class", "factorial", vec![5]),
            120
        );
    }
    #[test]
    fn int_factorial_10() {
        assert_eq!(
            run_static_int("Arithmetic.class", "factorial", vec![10]),
            3_628_800
        );
    }
    #[test]
    fn int_fibonacci_0() {
        assert_eq!(run_static_int("Arithmetic.class", "fibonacci", vec![0]), 0);
    }
    #[test]
    fn int_fibonacci_1() {
        assert_eq!(run_static_int("Arithmetic.class", "fibonacci", vec![1]), 1);
    }
    #[test]
    fn int_fibonacci_10() {
        assert_eq!(
            run_static_int("Arithmetic.class", "fibonacci", vec![10]),
            55
        );
    }
    #[test]
    fn int_sum_to_100() {
        assert_eq!(run_static_int("Arithmetic.class", "sumTo", vec![100]), 5050);
    }

    // ---- Phase 5: ClassContext + execute_class() tests ----

    fn load_class_context(class_name: &str) -> ClassContext {
        use duke_classfile::parse;
        let bytes = std::fs::read(fixture(class_name)).expect("fixture not found");
        let cf = parse(&bytes).expect("parse failed");
        build_class_context(&cf)
    }

    fn run_class_int(class_name: &str, method_name: &str, descriptor: &str, args: Vec<i32>) -> i32 {
        let ctx = load_class_context(class_name);
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        match execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            &entry_class,
            method_name,
            descriptor,
            &slots,
        )
        .expect("execute_class failed")
        {
            Some(Slot::Int(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn class_square() {
        assert_eq!(
            run_class_int("MathUtils.class", "square", "(I)I", vec![7]),
            49
        );
    }

    #[test]
    fn class_sum_of_squares_3_4() {
        assert_eq!(
            run_class_int("MathUtils.class", "sumOfSquares", "(II)I", vec![3, 4]),
            25
        );
    }

    #[test]
    fn class_sum_of_squares_5_12() {
        assert_eq!(
            run_class_int("MathUtils.class", "sumOfSquares", "(II)I", vec![5, 12]),
            169
        );
    }

    #[test]
    fn class_power_2_10() {
        assert_eq!(
            run_class_int("MathUtils.class", "power", "(II)I", vec![2, 10]),
            1024
        );
    }

    #[test]
    fn class_gcd_48_18() {
        assert_eq!(
            run_class_int("MathUtils.class", "gcd", "(II)I", vec![48, 18]),
            6
        );
    }

    #[test]
    fn class_method_not_found() {
        let ctx = load_class_context("MathUtils.class");
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        let err = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            &entry_class,
            "nonExistent",
            "(I)I",
            &[],
        )
        .unwrap_err();
        assert!(matches!(err, VmError::MethodNotFound { .. }));
    }

    // ---- Unit tests: parse_arg_count ----

    #[test]
    fn arg_count_empty() {
        assert_eq!(parse_arg_count("()V"), 0);
    }

    #[test]
    fn arg_count_single_int() {
        assert_eq!(parse_arg_count("(I)I"), 1);
    }

    #[test]
    fn arg_count_two_ints() {
        assert_eq!(parse_arg_count("(II)I"), 2);
    }

    #[test]
    fn arg_count_long_double() {
        assert_eq!(parse_arg_count("(JD)V"), 2);
    }

    #[test]
    fn arg_count_object_ref() {
        assert_eq!(parse_arg_count("(Ljava/lang/String;I)V"), 2);
    }

    #[test]
    fn arg_count_array() {
        assert_eq!(parse_arg_count("([II)I"), 2);
    }

    #[test]
    fn arg_count_mixed() {
        assert_eq!(parse_arg_count("(ILjava/lang/Object;Z)V"), 3);
    }

    // ---- Unit tests: resolve_methodref ----

    fn make_cp(entries: Vec<Option<CpEntry>>) -> Vec<Option<CpEntry>> {
        let mut cp = vec![None]; // slot 0 reserved
        cp.extend(entries);
        cp
    }

    #[test]
    fn resolve_methodref_valid() {
        use duke_classfile::types::CpIndex;
        let cp = make_cp(vec![
            Some(CpEntry::Methodref {
                class_index: CpIndex(2),
                name_and_type_index: CpIndex(3),
            }),
            Some(CpEntry::Class {
                name_index: CpIndex(6),
            }),
            Some(CpEntry::NameAndType {
                name_index: CpIndex(4),
                descriptor_index: CpIndex(5),
            }),
            Some(CpEntry::Utf8("square".to_string())),
            Some(CpEntry::Utf8("(I)I".to_string())),
            Some(CpEntry::Utf8("MathUtils".to_string())),
        ]);
        let (class_name, name, desc) = resolve_methodref(&cp, 1).unwrap();
        assert_eq!(class_name, "MathUtils");
        assert_eq!(name, "square");
        assert_eq!(desc, "(I)I");
    }

    #[test]
    fn resolve_methodref_invalid_index() {
        let cp = make_cp(vec![]);
        let err = resolve_methodref(&cp, 99).unwrap_err();
        assert!(matches!(err, VmError::InvalidMethodref { index: 99 }));
    }

    #[test]
    fn resolve_methodref_not_a_methodref() {
        let cp = make_cp(vec![Some(CpEntry::Utf8("not a methodref".to_string()))]);
        let err = resolve_methodref(&cp, 1).unwrap_err();
        assert!(matches!(err, VmError::InvalidMethodref { .. }));
    }

    // ---- Phase 6: object creation + field access ----

    #[test]
    fn point_sum_1_2_3_4() {
        assert_eq!(
            run_class_int("Point.class", "sumPoints", "(IIII)I", vec![1, 2, 3, 4]),
            10
        );
    }

    #[test]
    fn point_sum_3_4_0_0() {
        assert_eq!(
            run_class_int("Point.class", "sumPoints", "(IIII)I", vec![3, 4, 0, 0]),
            7
        );
    }

    #[test]
    fn point_sum_zeros() {
        assert_eq!(
            run_class_int("Point.class", "sumPoints", "(IIII)I", vec![0, 0, 0, 0]),
            0
        );
    }

    #[test]
    fn point_sum_symmetry() {
        let a = run_class_int("Point.class", "sumPoints", "(IIII)I", vec![1, 2, 3, 4]);
        let b = run_class_int("Point.class", "sumPoints", "(IIII)I", vec![3, 4, 1, 2]);
        assert_eq!(a, b);
    }

    // ---- Unit tests: resolve_fieldref ----

    #[test]
    fn resolve_fieldref_valid() {
        use duke_classfile::types::CpIndex;
        let cp = make_cp(vec![
            Some(CpEntry::Fieldref {
                class_index: CpIndex(2),
                name_and_type_index: CpIndex(3),
            }),
            Some(CpEntry::Class {
                name_index: CpIndex(6),
            }),
            Some(CpEntry::NameAndType {
                name_index: CpIndex(4),
                descriptor_index: CpIndex(5),
            }),
            Some(CpEntry::Utf8("x".to_string())),
            Some(CpEntry::Utf8("I".to_string())),
            Some(CpEntry::Utf8("Point".to_string())),
        ]);
        let (class_name, name, desc) = resolve_fieldref(&cp, 1).unwrap();
        assert_eq!(class_name, "Point");
        assert_eq!(name, "x");
        assert_eq!(desc, "I");
    }

    #[test]
    fn resolve_fieldref_invalid() {
        let cp = make_cp(vec![Some(CpEntry::Utf8("not a fieldref".to_string()))]);
        let err = resolve_fieldref(&cp, 1).unwrap_err();
        assert!(matches!(err, VmError::InvalidFieldref { .. }));
    }

    // ---- Phase 7: Arrays ----

    fn run_class_long(
        class_name: &str,
        method_name: &str,
        descriptor: &str,
        args: Vec<i32>,
    ) -> i64 {
        let ctx = load_class_context(class_name);
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        match execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            &entry_class,
            method_name,
            descriptor,
            &slots,
        )
        .expect("execute_class failed")
        {
            Some(Slot::Long(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    fn run_class_double(
        class_name: &str,
        method_name: &str,
        descriptor: &str,
        args: Vec<i32>,
    ) -> f64 {
        let ctx = load_class_context(class_name);
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        match execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            &entry_class,
            method_name,
            descriptor,
            &slots,
        )
        .expect("execute_class failed")
        {
            Some(Slot::Double(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn array_sum_5() {
        // sumArray(5) = 1+2+3+4+5 = 15
        assert_eq!(
            run_class_int("ArrayOps.class", "sumArray", "(I)I", vec![5]),
            15
        );
    }

    #[test]
    fn array_length() {
        assert_eq!(
            run_class_int("ArrayOps.class", "arrayLength", "(I)I", vec![7]),
            7
        );
    }

    #[test]
    fn array_sum_long() {
        // sumLongArray(3) = 0*1e6 + 1*1e6 + 2*1e6 = 3_000_000
        assert_eq!(
            run_class_long("ArrayOps.class", "sumLongArray", "(I)J", vec![3]),
            3_000_000
        );
    }

    #[test]
    fn array_first_double() {
        // firstDouble(3): a[0] = 0 * 0.5 = 0.0
        let result = run_class_double("ArrayOps.class", "firstDouble", "(I)D", vec![3]);
        assert!((result - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn array_empty_sum() {
        // sumArray(0) = sum of empty = 0
        assert_eq!(
            run_class_int("ArrayOps.class", "sumArray", "(I)I", vec![0]),
            0
        );
    }

    // ---- Phase 7: Array unit tests (no fixture needed) ----

    #[test]
    fn newarray_int_arraylength() {
        // newarray T_INT count=3 -> arraylength -> 3
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, Iconst3),
            (1, Newarray(ArrayType::Int)),
            (3, Arraylength),
            (4, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 3, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(3)));
    }

    #[test]
    fn newarray_iastore_iaload() {
        use duke_bytecode::Instruction::*;
        // int[] a = new int[1]; a[0] = 42; return a[0];
        let instrs = vec![
            (0, Iconst1),
            (1, Newarray(ArrayType::Int)),
            (3, Dup),
            (4, Iconst0),
            (5, Bipush(42)),
            (7, Iastore),
            (8, Iconst0),
            (9, Iaload),
            (10, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 4, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(42)));
    }

    #[test]
    fn newarray_int_bounds_error() {
        use duke_bytecode::Instruction::*;
        // new int[1], then iaload at index 5 -> ArrayIndexOutOfBounds
        let instrs = vec![
            (0, Iconst1),
            (1, Newarray(ArrayType::Int)),
            (3, Bipush(5)),
            (5, Iaload),
            (6, Ireturn),
        ];
        let err = execute(&instrs, &[], vec![], 3, 0).unwrap_err();
        assert!(matches!(
            err,
            VmError::ArrayIndexOutOfBounds {
                index: 5,
                length: 1
            }
        ));
    }

    #[test]
    fn newarray_negative_size() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, IconstM1),
            (1, Newarray(ArrayType::Int)),
            (3, Arraylength),
            (4, Ireturn),
        ];
        let err = execute(&instrs, &[], vec![], 2, 0).unwrap_err();
        assert!(matches!(err, VmError::NegativeArraySize { size: -1 }));
    }

    #[test]
    fn athrow_propagates_as_java_exception() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![(0, Iconst1), (1, Newarray(ArrayType::Int)), (3, Athrow)];
        let err = execute(&instrs, &[], vec![], 2, 0).unwrap_err();
        assert!(matches!(err, VmError::JavaException { .. }));
    }

    // ---- Phase 8: Exceptions ----

    #[test]
    fn exception_catch_simple() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "catchSimple", "()I", vec![]),
            42
        );
    }

    #[test]
    fn exception_uncaught_propagates() {
        let ctx = load_class_context("ExceptionTest.class");
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            &entry_class,
            "uncaught",
            "()I",
            &[],
        );
        let err = result.unwrap_err();
        assert!(matches!(err, VmError::JavaException { .. }));
    }

    #[test]
    fn exception_finally_block() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "finallyBlock", "()I", vec![]),
            11 // 10 + 1
        );
    }

    #[test]
    fn exception_catch_from_callee() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "catchFromCallee", "()I", vec![]),
            99
        );
    }

    // ---- Phase 8: Switch statements ----

    #[test]
    fn switch_dense_case0() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "switchDense", "(I)I", vec![0]),
            10
        );
    }

    #[test]
    fn switch_dense_case2() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "switchDense", "(I)I", vec![2]),
            30
        );
    }

    #[test]
    fn switch_dense_default() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "switchDense", "(I)I", vec![99]),
            -1
        );
    }

    #[test]
    fn switch_sparse_case200() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "switchSparse", "(I)I", vec![200]),
            2
        );
    }

    #[test]
    fn switch_sparse_default() {
        assert_eq!(
            run_class_int("ExceptionTest.class", "switchSparse", "(I)I", vec![999]),
            0
        );
    }

    // ---- Phase 8: Switch (unit tests) ----

    #[test]
    fn tableswitch_match() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, Iconst1),
            (
                1,
                Tableswitch {
                    default: 100,
                    low: 0,
                    high: 2,
                    offsets: vec![10, 20, 30],
                },
            ),
            (11, Bipush(10)),
            (13, Ireturn),
            (21, Bipush(20)),
            (23, Ireturn),
            (31, Bipush(30)),
            (33, Ireturn),
            (101, Bipush(-1)),
            (103, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(20))); // case 1
    }

    #[test]
    fn tableswitch_default() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, Bipush(99)),
            (
                2,
                Tableswitch {
                    default: 100,
                    low: 0,
                    high: 2,
                    offsets: vec![10, 20, 30],
                },
            ),
            (12, Bipush(10)),
            (14, Ireturn),
            (22, Bipush(20)),
            (24, Ireturn),
            (32, Bipush(30)),
            (34, Ireturn),
            (102, Bipush(-1)),
            (104, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(-1)));
    }

    #[test]
    fn lookupswitch_match() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, Sipush(200)),
            (
                3,
                Lookupswitch {
                    default: 100,
                    pairs: vec![(100, 10), (200, 20), (300, 30)],
                },
            ),
            (13, Bipush(1)),
            (15, Ireturn),
            (23, Bipush(2)),
            (25, Ireturn),
            (33, Bipush(3)),
            (35, Ireturn),
            (103, Bipush(0)),
            (105, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    #[test]
    fn lookupswitch_default() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, Sipush(999)),
            (
                3,
                Lookupswitch {
                    default: 100,
                    pairs: vec![(100, 10), (200, 20), (300, 30)],
                },
            ),
            (13, Bipush(1)),
            (15, Ireturn),
            (23, Bipush(2)),
            (25, Ireturn),
            (33, Bipush(3)),
            (35, Ireturn),
            (103, Bipush(0)),
            (105, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 2, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(0)));
    }

    // ---- Phase 9: String constants ----

    #[test]
    fn string_non_null() {
        let result = run_class_int("StringAndTypes.class", "stringNonNull", "()I", vec![]);
        assert_eq!(result, 1);
    }

    #[test]
    fn string_intern() {
        let result = run_class_int("StringAndTypes.class", "stringIntern", "()I", vec![]);
        assert_eq!(result, 1);
    }

    // ---- Phase 9: instanceof ----

    #[test]
    fn instanceof_match() {
        let result = run_class_int("StringAndTypes.class", "instanceOfMatch", "()I", vec![]);
        assert_eq!(result, 1);
    }

    #[test]
    fn instanceof_mismatch() {
        let result = run_class_int("StringAndTypes.class", "instanceOfMismatch", "()I", vec![]);
        assert_eq!(result, 0);
    }

    #[test]
    fn instanceof_null() {
        let result = run_class_int("StringAndTypes.class", "instanceOfNull", "()I", vec![]);
        assert_eq!(result, 0);
    }

    // ---- Phase 9: checkcast ----

    #[test]
    fn checkcast_ok() {
        let result = run_class_int("StringAndTypes.class", "checkcastOk", "()I", vec![]);
        assert_eq!(result, 42);
    }

    // ---- Phase 9: Reference comparison ----

    #[test]
    fn ref_equal() {
        let result = run_class_int("StringAndTypes.class", "refEqual", "()I", vec![]);
        assert_eq!(result, 1);
    }

    #[test]
    fn ref_not_equal() {
        let result = run_class_int("StringAndTypes.class", "refNotEqual", "()I", vec![]);
        assert_eq!(result, 1);
    }

    // ---- Phase 9: if_acmpeq / if_acmpne unit tests ----

    #[test]
    fn if_acmpeq_same_ref() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, Iconst1),
            (1, Newarray(ArrayType::Int)),
            (3, Dup),
            (4, IfAcmpeq(10)),
            (7, Iconst0),
            (8, Ireturn),
            (14, Iconst1),
            (15, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 3, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn if_acmpne_different_refs() {
        use duke_bytecode::Instruction::*;
        let instrs = vec![
            (0, Iconst1),
            (1, Newarray(ArrayType::Int)),
            (3, Iconst1),
            (4, Newarray(ArrayType::Int)),
            (6, IfAcmpne(10)),
            (9, Iconst0),
            (10, Ireturn),
            (16, Iconst1),
            (17, Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 3, 0).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    // ---- Phase 10: Cross-class dispatch ----

    fn run_cross_class_int(
        class_files: &[&str],
        entry_class: &str,
        method_name: &str,
        descriptor: &str,
        args: Vec<i32>,
    ) -> i32 {
        let mut registry = ClassRegistry::new();
        for name in class_files {
            let ctx = load_class_context(name);
            registry.register(ctx);
        }
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let slots: Vec<Slot> = args.into_iter().map(Slot::Int).collect();
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        match execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            entry_class,
            method_name,
            descriptor,
            &slots,
        )
        .expect("execute_class failed")
        {
            Some(Slot::Int(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn cross_class_add() {
        assert_eq!(
            run_cross_class_int(
                &["CrossCall.class", "Callee.class"],
                "CrossCall",
                "callAdd",
                "(II)I",
                vec![3, 4],
            ),
            7
        );
    }

    #[test]
    fn cross_class_double() {
        assert_eq!(
            run_cross_class_int(
                &["CrossCall.class", "Callee.class"],
                "CrossCall",
                "callDouble",
                "(I)I",
                vec![5],
            ),
            10
        );
    }

    #[test]
    fn cross_class_chain() {
        assert_eq!(
            run_cross_class_int(
                &["CrossCall.class", "Callee.class"],
                "CrossCall",
                "chainCall",
                "(I)I",
                vec![3],
            ),
            9
        );
    }

    #[test]
    fn cross_class_add_negated() {
        assert_eq!(
            run_cross_class_int(
                &["CrossCall.class", "Callee.class"],
                "CrossCall",
                "addNegated",
                "(I)I",
                vec![5],
            ),
            0
        );
    }

    #[test]
    fn cross_class_pair_sum() {
        assert_eq!(
            run_cross_class_int(
                &["PairUser.class", "Pair.class"],
                "PairUser",
                "makePairSum",
                "(II)I",
                vec![3, 7],
            ),
            10
        );
    }

    #[test]
    fn cross_class_pair_diff() {
        assert_eq!(
            run_cross_class_int(
                &["PairUser.class", "Pair.class"],
                "PairUser",
                "makePairDiff",
                "(II)I",
                vec![10, 3],
            ),
            7
        );
    }

    #[test]
    fn cross_class_two_pairs() {
        assert_eq!(
            run_cross_class_int(
                &["PairUser.class", "Pair.class"],
                "PairUser",
                "twoPairsSum",
                "(IIII)I",
                vec![1, 2, 3, 4],
            ),
            10
        );
    }

    #[test]
    fn native_registry_stores_and_retrieves() {
        #[allow(clippy::unnecessary_wraps)]
        fn dummy_handler(
            _args: &[Slot],
            _heap: &mut duke_gc::Heap,
            _out: &mut dyn std::io::Write,
            _control: &mut NativeControl,
        ) -> VmResult<Option<Slot>> {
            Ok(Some(Slot::Int(99)))
        }
        let mut natives = NativeRegistry::new();
        natives.register("Foo", "bar", "(I)I", dummy_handler);
        let handler = natives.get("Foo", "bar", "(I)I");
        assert!(handler.is_some());
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let result = handler.unwrap()(
            &[Slot::Int(1)],
            &mut heap,
            &mut out,
            &mut NativeControl::default(),
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(99)));
    }

    #[test]
    fn native_registry_returns_none_for_missing() {
        let natives = NativeRegistry::new();
        assert!(natives.get("Foo", "bar", "(I)I").is_none());
    }

    // ---- Phase 11: native println tests ----

    fn load_hello_class() -> ClassContext {
        let bytes = std::fs::read(fixture("Hello.class")).expect("Hello.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn hello_greet_prints_to_output() {
        let ctx = load_hello_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "Hello",
            "greet",
            "()V",
            &[],
        )
        .unwrap();
        assert_eq!(result, None);
        assert_eq!(String::from_utf8_lossy(&out), "Hello, Duke!\n");
    }

    #[test]
    fn hello_print_num() {
        let ctx = load_hello_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "Hello",
            "printNum",
            "(I)V",
            &[Slot::Int(42)],
        )
        .unwrap();
        assert_eq!(result, None);
        assert_eq!(String::from_utf8_lossy(&out), "42\n");
    }

    #[test]
    fn hello_greet_and_return() {
        let ctx = load_hello_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "Hello",
            "greetAndReturn",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(42)));
        assert_eq!(String::from_utf8_lossy(&out), "Greetings!\n");
    }

    #[test]
    fn hello_blank_line() {
        let ctx = load_hello_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "Hello",
            "blankLine",
            "()V",
            &[],
        )
        .unwrap();
        assert_eq!(result, None);
        assert_eq!(String::from_utf8_lossy(&out), "\n");
    }

    // ---- Phase 12: stack manipulation tests ----

    #[test]
    fn stack_ops_dup_x1() {
        // dup_x1: ..., v2, v1 → ..., v1, v2, v1
        let instrs = vec![
            (0, Instruction::Iconst2), // push 2 (v2)
            (1, Instruction::Iconst3), // push 3 (v1)
            (2, Instruction::DupX1),   // → 3, 2, 3
            (3, Instruction::Iadd),    // → 3, 5
            (4, Instruction::Iadd),    // → 8
            (5, Instruction::Ireturn),
        ];
        let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
        assert_eq!(r, Some(Slot::Int(8)));
    }

    #[test]
    fn stack_ops_dup_x2() {
        // dup_x2: ..., v3, v2, v1 → ..., v1, v3, v2, v1
        let instrs = vec![
            (0, Instruction::Iconst1), // push 1 (v3)
            (1, Instruction::Iconst2), // push 2 (v2)
            (2, Instruction::Iconst3), // push 3 (v1)
            (3, Instruction::DupX2),   // → 3, 1, 2, 3
            (4, Instruction::Iadd),    // → 3, 1, 5
            (5, Instruction::Iadd),    // → 3, 6
            (6, Instruction::Iadd),    // → 9
            (7, Instruction::Ireturn),
        ];
        let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
        assert_eq!(r, Some(Slot::Int(9)));
    }

    #[test]
    fn stack_ops_dup2() {
        // dup2: ..., v2, v1 → ..., v2, v1, v2, v1
        let instrs = vec![
            (0, Instruction::Iconst4), // push 4 (v2)
            (1, Instruction::Iconst5), // push 5 (v1)
            (2, Instruction::Dup2),    // → 4, 5, 4, 5
            (3, Instruction::Iadd),    // → 4, 5, 9
            (4, Instruction::Iadd),    // → 4, 14
            (5, Instruction::Iadd),    // → 18
            (6, Instruction::Ireturn),
        ];
        let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
        assert_eq!(r, Some(Slot::Int(18)));
    }

    #[test]
    fn stack_ops_dup2_x1() {
        // dup2_x1: ..., v3, v2, v1 → ..., v2, v1, v3, v2, v1
        let instrs = vec![
            (0, Instruction::Iconst1), // 1 (v3)
            (1, Instruction::Iconst2), // 2 (v2)
            (2, Instruction::Iconst3), // 3 (v1)
            (3, Instruction::Dup2X1),  // → 2, 3, 1, 2, 3
            (4, Instruction::Iadd),    // → 2, 3, 1, 5
            (5, Instruction::Iadd),    // → 2, 3, 6
            (6, Instruction::Iadd),    // → 2, 9
            (7, Instruction::Iadd),    // → 11
            (8, Instruction::Ireturn),
        ];
        let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
        assert_eq!(r, Some(Slot::Int(11)));
    }

    #[test]
    fn stack_ops_dup2_x2() {
        // dup2_x2: ..., v4, v3, v2, v1 → ..., v2, v1, v4, v3, v2, v1
        let instrs = vec![
            (0, Instruction::Iconst1), // 1 (v4)
            (1, Instruction::Iconst2), // 2 (v3)
            (2, Instruction::Iconst3), // 3 (v2)
            (3, Instruction::Iconst4), // 4 (v1)
            (4, Instruction::Dup2X2),  // → 3, 4, 1, 2, 3, 4
            (5, Instruction::Iadd),    // → 3, 4, 1, 2, 7
            (6, Instruction::Iadd),    // → 3, 4, 1, 9
            (7, Instruction::Iadd),    // → 3, 4, 10
            (8, Instruction::Iadd),    // → 3, 14
            (9, Instruction::Iadd),    // → 17
            (10, Instruction::Ireturn),
        ];
        let r = execute(&instrs, &[], vec![], 10, 1).unwrap();
        assert_eq!(r, Some(Slot::Int(17)));
    }

    // ---- Phase 12: static initializer tests ----

    fn load_static_init_class() -> ClassContext {
        let bytes = std::fs::read(fixture("StaticInit.class")).expect("StaticInit.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    fn fixtures_loader() -> duke_loader::DirectoryLoader {
        duke_loader::DirectoryLoader::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("tests")
                .join("fixtures"),
        )
    }

    #[test]
    fn clinit_initializes_static_field_x() {
        let ctx = load_static_init_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = fixtures_loader();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StaticInit",
            "getX",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(42)));
    }

    #[test]
    fn clinit_initializes_dependent_field_y() {
        let ctx = load_static_init_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = fixtures_loader();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StaticInit",
            "getY",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(50)));
    }

    #[test]
    fn clinit_runs_static_block() {
        let ctx = load_static_init_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = fixtures_loader();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StaticInit",
            "getZ",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(92)));
    }

    #[test]
    fn clinit_sum_all_statics() {
        let ctx = load_static_init_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = fixtures_loader();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StaticInit",
            "sum",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(184)));
    }

    // ---- Phase 12: stack ops integration tests ----

    fn load_stack_ops_class() -> ClassContext {
        let bytes = std::fs::read(fixture("StackOps.class")).expect("StackOps.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn stack_ops_array_store_dup() {
        let ctx = load_stack_ops_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = fixtures_loader();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StackOps",
            "arrayStoreDup",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(60)));
    }

    #[test]
    fn stack_ops_multi_assign() {
        let ctx = load_stack_ops_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = fixtures_loader();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StackOps",
            "multiAssign",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(10)));
    }

    // ---- Phase 13: class hierarchy tests ----

    fn load_hierarchy_class() -> ClassContext {
        let bytes = std::fs::read(fixture("Hierarchy.class")).expect("Hierarchy.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    fn load_exception_hierarchy_class() -> ClassContext {
        let bytes =
            std::fs::read(fixture("ExceptionHierarchy.class")).expect("ExceptionHierarchy.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    fn load_string_ops_class() -> ClassContext {
        let bytes = std::fs::read(fixture("StringOps.class")).expect("StringOps.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn hierarchy_instanceof_object() {
        let ctx = load_hierarchy_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "Hierarchy",
            "instanceOfObject",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn hierarchy_cast_to_object() {
        let ctx = load_hierarchy_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "Hierarchy",
            "castToObject",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn hierarchy_null_instanceof() {
        let ctx = load_hierarchy_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "Hierarchy",
            "nullInstanceOf",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(0)));
    }

    // ---- Phase 13: exception hierarchy tests ----

    #[test]
    fn exception_hierarchy_catch_parent() {
        let ctx = load_exception_hierarchy_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "ExceptionHierarchy",
            "catchParent",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn exception_hierarchy_catch_exact() {
        let ctx = load_exception_hierarchy_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "ExceptionHierarchy",
            "catchExact",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    #[test]
    fn exception_hierarchy_catch_wrong_then_right() {
        let ctx = load_exception_hierarchy_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "ExceptionHierarchy",
            "catchWrongThenRight",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(3)));
    }

    #[test]
    fn exception_hierarchy_catch_grandparent() {
        let ctx = load_exception_hierarchy_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "ExceptionHierarchy",
            "catchGrandparent",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(4)));
    }

    // ---- Phase 13: string ops tests ----

    #[test]
    fn string_ops_length() {
        let ctx = load_string_ops_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringOps",
            "stringLength",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(5)));
    }

    #[test]
    fn string_ops_equals() {
        let ctx = load_string_ops_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringOps",
            "stringEquals",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops_not_equals() {
        let ctx = load_string_ops_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringOps",
            "stringNotEquals",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(0)));
    }

    #[test]
    fn string_ops_char_at() {
        let ctx = load_string_ops_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringOps",
            "charAtOne",
            "()I",
            &[],
        )
        .unwrap();
        // 'e' = 101
        assert_eq!(result, Some(Slot::Int(101)));
    }

    // ---- Phase 14: interface tests ----

    fn load_class(name: &str) -> ClassContext {
        let bytes = std::fs::read(fixture(name)).unwrap_or_else(|_| panic!("{name}"));
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn interface_call_simple() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("InterfaceTest.class"));
        registry.register(load_class("InterfaceTest$Adder.class"));
        registry.register(load_class("InterfaceTest$SimpleAdder.class"));
        registry.register(load_class("InterfaceTest$DoubleAdder.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InterfaceTest",
            "callSimple",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(7)));
    }

    #[test]
    fn interface_call_double() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("InterfaceTest.class"));
        registry.register(load_class("InterfaceTest$Adder.class"));
        registry.register(load_class("InterfaceTest$SimpleAdder.class"));
        registry.register(load_class("InterfaceTest$DoubleAdder.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InterfaceTest",
            "callDouble",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(14)));
    }

    #[test]
    fn interface_polymorphic_simple() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("InterfaceTest.class"));
        registry.register(load_class("InterfaceTest$Adder.class"));
        registry.register(load_class("InterfaceTest$SimpleAdder.class"));
        registry.register(load_class("InterfaceTest$DoubleAdder.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InterfaceTest",
            "polymorphic",
            "(I)I",
            &[Slot::Int(0)],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(8)));
    }

    #[test]
    fn interface_polymorphic_double() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("InterfaceTest.class"));
        registry.register(load_class("InterfaceTest$Adder.class"));
        registry.register(load_class("InterfaceTest$SimpleAdder.class"));
        registry.register(load_class("InterfaceTest$DoubleAdder.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InterfaceTest",
            "polymorphic",
            "(I)I",
            &[Slot::Int(1)],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(16)));
    }

    // ---- Phase 14: multi-dimensional array tests ----

    #[test]
    fn multiarray_sum2d() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("MultiArray.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "MultiArray",
            "sum2d",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(21)));
    }

    #[test]
    fn multiarray_dimensions() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("MultiArray.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "MultiArray",
            "dimensions",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(12)));
    }

    // ---- Phase 14: native method tests ----

    #[test]
    fn more_natives_object_hashcode() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("MoreNatives.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "MoreNatives",
            "objectHashCode",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn more_natives_value_of_int() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("MoreNatives.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "MoreNatives",
            "valueOfInt",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    #[test]
    fn more_natives_print_no_newline() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("MoreNatives.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "MoreNatives",
            "printNoNewline",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        assert_eq!(String::from_utf8_lossy(&out), "ABCD\n");
    }

    // ---- Phase 15: inherited method tests ----

    fn load_inherited_method_classes(registry: &mut ClassRegistry) {
        registry.register(load_class("InheritedMethod.class"));
        registry.register(load_class("InheritedMethod$Animal.class"));
        registry.register(load_class("InheritedMethod$Dog.class"));
        registry.register(load_class("InheritedMethod$Puppy.class"));
    }

    #[test]
    fn inherited_call_inherited() {
        let mut registry = ClassRegistry::new();
        load_inherited_method_classes(&mut registry);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InheritedMethod",
            "callInherited",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(10)));
    }

    #[test]
    fn inherited_call_overridden() {
        let mut registry = ClassRegistry::new();
        load_inherited_method_classes(&mut registry);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InheritedMethod",
            "callOverridden",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    #[test]
    fn inherited_call_deep_inherited() {
        let mut registry = ClassRegistry::new();
        load_inherited_method_classes(&mut registry);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InheritedMethod",
            "callDeepInherited",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(10)));
    }

    #[test]
    fn inherited_call_deep_overridden() {
        let mut registry = ClassRegistry::new();
        load_inherited_method_classes(&mut registry);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "InheritedMethod",
            "callDeepOverridden",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    // ---- Phase 15: string methods tests ----

    #[test]
    fn string_methods_substring() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testSubstring",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(5)));
    }

    #[test]
    fn string_methods_substring_range() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testSubstringRange",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(5)));
    }

    #[test]
    fn string_methods_indexof() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testIndexOf",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(5)));
    }

    #[test]
    fn string_methods_indexof_not_found() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testIndexOfNotFound",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(-1)));
    }

    #[test]
    fn string_methods_contains() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testContains",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_methods_isempty() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testIsEmpty",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_methods_compareto() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testCompareTo",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_methods_startswith() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testStartsWith",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_methods_endswith() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testEndsWith",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_methods_trim() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testTrim",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(2)));
    }

    #[test]
    fn string_methods_tochararray() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("StringMethods.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut sink: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "StringMethods",
            "testToCharArray",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(131)));
    }

    // ---- Phase 15: main entry point tests ----

    #[test]
    fn main_hello_no_args() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("MainHello.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();

        // Build empty String[] array on the heap.
        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
        let main_args = vec![Slot::Reference(Some(arr_ref))];

        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "MainHello",
            "main",
            "([Ljava/lang/String;)V",
            &main_args,
        )
        .unwrap();
        assert_eq!(result, None);
        assert_eq!(String::from_utf8_lossy(&out), "no args\n");
    }

    #[test]
    fn main_hello_with_args() {
        let mut registry = ClassRegistry::new();
        registry.register(load_class("MainHello.class"));
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();

        // Build String[] with ["Alice", "Bob"] on the heap.
        let alice_ref = heap.allocate_string("Alice".to_string());
        let bob_ref = heap.allocate_string("Bob".to_string());
        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 2);
        heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(alice_ref));
        heap.get_mut(arr_ref).unwrap().fields[1] = Slot::Reference(Some(bob_ref));
        let main_args = vec![Slot::Reference(Some(arr_ref))];

        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "MainHello",
            "main",
            "([Ljava/lang/String;)V",
            &main_args,
        )
        .unwrap();
        assert_eq!(result, None);
        assert_eq!(String::from_utf8_lossy(&out), "Alice\nBob\n");
    }

    // ---- Phase 16: PrintAll integration tests ----

    fn load_print_all_class() -> ClassContext {
        let bytes = std::fs::read(fixture("PrintAll.class")).expect("PrintAll.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn print_all_long() {
        let ctx = load_print_all_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "PrintAll",
            "printLong",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        assert!(
            String::from_utf8_lossy(&out).contains("9876543210"),
            "stdout should contain 9876543210, got: {}",
            String::from_utf8_lossy(&out)
        );
    }

    #[test]
    fn print_all_double() {
        let ctx = load_print_all_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "PrintAll",
            "printDouble",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        assert!(
            String::from_utf8_lossy(&out).contains("3.14"),
            "stdout should contain 3.14, got: {}",
            String::from_utf8_lossy(&out)
        );
    }

    #[test]
    fn print_all_float() {
        let ctx = load_print_all_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "PrintAll",
            "printFloat",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        assert!(
            String::from_utf8_lossy(&out).contains("2.5"),
            "stdout should contain 2.5, got: {}",
            String::from_utf8_lossy(&out)
        );
    }

    #[test]
    fn print_all_boolean() {
        let ctx = load_print_all_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "PrintAll",
            "printBoolean",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        let output = String::from_utf8_lossy(&out);
        assert!(
            output.contains("true") && output.contains("false"),
            "stdout should contain true and false, got: {output}",
        );
    }

    #[test]
    fn print_all_char() {
        let ctx = load_print_all_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "PrintAll",
            "printChar",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        assert!(
            String::from_utf8_lossy(&out).contains('Z'),
            "stdout should contain Z, got: {}",
            String::from_utf8_lossy(&out)
        );
    }

    #[test]
    fn print_all_mixed() {
        let ctx = load_print_all_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "PrintAll",
            "printMixed",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
        assert_eq!(String::from_utf8_lossy(&out), "val=42\n");
    }

    // ---- Phase 16: ParseArgs integration tests ----

    fn load_parse_args_class() -> ClassContext {
        let bytes = std::fs::read(fixture("ParseArgs.class")).expect("ParseArgs.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn parse_args_parse_int() {
        let ctx = load_parse_args_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        // Build String[] with ["123"]
        let s_ref = heap.allocate_string("123".to_string());
        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 1);
        heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(s_ref));
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "parseInt",
            "([Ljava/lang/String;)I",
            &[Slot::Reference(Some(arr_ref))],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(123)));
    }

    #[test]
    fn parse_args_add_parsed() {
        let ctx = load_parse_args_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        // Build String[] with ["10", "20"]
        let s1 = heap.allocate_string("10".to_string());
        let s2 = heap.allocate_string("20".to_string());
        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 2);
        heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(s1));
        heap.get_mut(arr_ref).unwrap().fields[1] = Slot::Reference(Some(s2));
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "addParsed",
            "([Ljava/lang/String;)I",
            &[Slot::Reference(Some(arr_ref))],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(30)));
    }

    #[test]
    fn parse_args_valueof() {
        let ctx = load_parse_args_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "valueOf",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(42)));
    }

    #[test]
    fn parse_args_math_max() {
        let ctx = load_parse_args_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "mathMax",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(7)));
    }

    #[test]
    fn parse_args_math_min() {
        let ctx = load_parse_args_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "mathMin",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(3)));
    }

    #[test]
    fn parse_args_math_abs() {
        let ctx = load_parse_args_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "mathAbs",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(5)));
    }

    // ---- Phase 16: StringConcat integration tests ----

    fn load_string_concat_class() -> ClassContext {
        let bytes = std::fs::read(fixture("StringConcat.class")).expect("StringConcat.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn string_concat_length() {
        let ctx = load_string_concat_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcat",
            "concatLength",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(10)));
    }

    #[test]
    fn string_concat_bool_to_string() {
        let ctx = load_string_concat_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcat",
            "boolToString",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(4)));
    }

    #[test]
    fn string_concat_long_to_string() {
        let ctx = load_string_concat_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcat",
            "longToString",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(3)));
    }

    #[test]
    fn string_concat_char_to_string() {
        let ctx = load_string_concat_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcat",
            "charToString",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_concat_double_to_string() {
        let ctx = load_string_concat_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcat",
            "doubleToString",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    // ---- Phase 17: StringConcatFactory (invokedynamic) ----

    fn load_string_concat_test_class() -> ClassContext {
        let bytes =
            std::fs::read(fixture("StringConcatTest.class")).expect("StringConcatTest.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn string_concat_simple() {
        let ctx = load_string_concat_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcatTest",
            "testSimple",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_concat_int() {
        let ctx = load_string_concat_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcatTest",
            "testInt",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_concat_chain() {
        let ctx = load_string_concat_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcatTest",
            "testChain",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_concat_boolean() {
        let ctx = load_string_concat_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcatTest",
            "testBoolean",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_concat_empty() {
        let ctx = load_string_concat_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringConcatTest",
            "testEmpty",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    // ---- Phase 17: LambdaMetafactory ----

    fn load_lambda_test_class() -> ClassContext {
        let bytes = std::fs::read(fixture("LambdaTest.class")).expect("LambdaTest.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn lambda_simple_no_capture() {
        let ctx = load_lambda_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "LambdaTest",
            "testDouble",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(10)));
    }

    #[test]
    fn lambda_with_capture() {
        let ctx = load_lambda_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "LambdaTest",
            "testCapture",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(107)));
    }

    #[test]
    fn lambda_method_reference() {
        let ctx = load_lambda_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "LambdaTest",
            "testMethodRef",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(-42)));
    }

    #[test]
    fn lambda_multi_capture() {
        let ctx = load_lambda_test_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "LambdaTest",
            "testMultiCapture",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(33)));
    }

    // ---- MonitorAndAbstract fixture ----

    fn load_monitor_class() -> ClassContext {
        let bytes =
            std::fs::read(fixture("MonitorAndAbstract.class")).expect("MonitorAndAbstract.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn monitor_sync_block() {
        let ctx = load_monitor_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "MonitorAndAbstract",
            "syncBlock",
            "(I)I",
            &[Slot::Int(7)],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(14)));
    }

    #[test]
    fn monitor_sync_method() {
        let ctx = load_monitor_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "MonitorAndAbstract",
            "syncMethod",
            "(I)I",
            &[Slot::Int(5)],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(15)));
    }

    #[test]
    fn monitor_nested_sync() {
        let ctx = load_monitor_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "MonitorAndAbstract",
            "nestedSync",
            "(I)I",
            &[Slot::Int(10)],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(15)));
    }

    // ---- ExtendedMath integration tests ----

    fn load_extended_math_class() -> ClassContext {
        let bytes = std::fs::read(fixture("ExtendedMath.class")).expect("ExtendedMath.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn extended_math_sqrt() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testSqrt",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_pow() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testPow",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_floor_ceil() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testFloorCeil",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_round() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testRound",
            "()J",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Long(4)));
    }

    #[test]
    fn extended_math_abs_long() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testAbsLong",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_abs_double() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testAbsDouble",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_max_long() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testMaxLong",
            "()J",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Long(200)));
    }

    #[test]
    fn extended_math_min_long() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testMinLong",
            "()J",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Long(100)));
    }

    #[test]
    fn extended_math_max_double() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testMaxDouble",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_parse_long() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testParseLong",
            "()J",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Long(9_876_543_210)));
    }

    #[test]
    fn extended_math_parse_double() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testParseDouble",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_parse_float() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testParseFloat",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_parse_boolean() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testParseBoolean",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_long_valueof() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testLongValueOf",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn extended_math_constants() {
        let ctx = load_extended_math_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ExtendedMath",
            "testMathConstants",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    // ---- StringOps2 integration tests ----

    fn load_string_ops2_class() -> ClassContext {
        let bytes = std::fs::read(fixture("StringOps2.class")).expect("StringOps2.class");
        let cf = duke_classfile::parse(&bytes).unwrap();
        build_class_context(&cf)
    }

    #[test]
    fn string_ops2_to_upper_case() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testToUpperCase",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_to_lower_case() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testToLowerCase",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_replace_char() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testReplace",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_replace_string() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testReplaceString",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_split() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testSplit",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_hashcode() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testHashCode",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_tostring() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testToStringIdentity",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn string_ops2_replace_charsequence() {
        let ctx = load_string_ops2_class();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "StringOps2",
            "testReplaceCharSequence",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn class_literal_non_null() {
        assert_eq!(
            run_class_int("ClassLiteral.class", "testStringClass", "()I", vec![]),
            1
        );
    }

    #[test]
    fn class_literal_self_ref() {
        assert_eq!(
            run_class_int("ClassLiteral.class", "testPrimitiveClass", "()I", vec![]),
            1
        );
    }

    #[test]
    fn class_literal_interning() {
        assert_eq!(
            run_class_int("ClassLiteral.class", "testClassInterning", "()I", vec![]),
            1
        );
    }

    // ---- Phase 19: Enum integration tests ----

    /// Helper that loads a class, calls `bootstrap_stdlib`, and runs a static method.
    fn run_bootstrap_int(class_name: &str, method_name: &str, descriptor: &str) -> i32 {
        let ctx = load_class_context(class_name);
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        match execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            &entry_class,
            method_name,
            descriptor,
            &[],
        )
        .expect("execute_class failed")
        {
            Some(Slot::Int(v)) => v,
            other => panic!("unexpected result: {other:?}"),
        }
    }

    fn run_bootstrap_with_string_args(
        class_name: &str,
        method_name: &str,
        descriptor: &str,
        args: &[String],
    ) -> VmResult<Option<Slot>> {
        let ctx = load_class_context(class_name);
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let arg_slots: Vec<Slot> = args
            .iter()
            .map(|arg| Slot::Reference(Some(heap.allocate_string(arg.clone()))))
            .collect();
        let mut out: Vec<u8> = Vec::new();
        execute_class_to_completion(
            &mut registry,
            loader,
            &mut heap,
            &mut out,
            &entry_class,
            method_name,
            descriptor,
            &arg_slots,
        )
    }

    fn run_bootstrap_with_output(
        class_name: &str,
        method_name: &str,
        descriptor: &str,
    ) -> VmResult<(Option<Slot>, Vec<String>)> {
        let ctx = load_class_context(class_name);
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class_to_completion(
            &mut registry,
            loader,
            &mut heap,
            &mut out,
            &entry_class,
            method_name,
            descriptor,
            &[],
        )?;
        let lines = String::from_utf8(out)
            .expect("captured output is utf8")
            .lines()
            .map(std::string::ToString::to_string)
            .collect();
        Ok((result, lines))
    }

    struct TempCleanup(std::path::PathBuf);

    impl Drop for TempCleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn make_temp_root(prefix: &str) -> (std::path::PathBuf, TempCleanup) {
        let unique = format!(
            "{prefix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock is before UNIX_EPOCH")
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&root).expect("create temp root");
        let cleanup = TempCleanup(root.clone());
        (root, cleanup)
    }

    #[cfg(feature = "telemetry")]
    fn run_fixture(
        class_name: &str,
        method_name: &str,
        descriptor: &str,
    ) -> (Option<Slot>, ClassRegistry) {
        let ctx = load_class_context(class_name);
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            &entry_class,
            method_name,
            descriptor,
            &[],
        )
        .expect("execute_class failed");
        (result, registry)
    }

    #[test]
    fn enum_ordinal() {
        assert_eq!(
            run_bootstrap_int("SimpleEnum.class", "testOrdinal", "()I"),
            1
        );
    }

    #[test]
    fn enum_name_length() {
        assert_eq!(run_bootstrap_int("SimpleEnum.class", "testName", "()I"), 3);
    }

    #[test]
    fn enum_values_length() {
        assert_eq!(
            run_bootstrap_int("SimpleEnum.class", "testValues", "()I"),
            3
        );
    }

    #[test]
    fn enum_valueof() {
        assert_eq!(
            run_bootstrap_int("SimpleEnum.class", "testValueOf", "()I"),
            2
        );
    }

    #[test]
    fn enum_switch() {
        assert_eq!(
            run_bootstrap_int("SimpleEnum.class", "testSwitch", "()I"),
            20
        );
    }

    #[test]
    fn enum_equality() {
        assert_eq!(
            run_bootstrap_int("SimpleEnum.class", "testEquality", "()I"),
            1
        );
    }

    // ---- StringBuilder tests ----

    #[test]
    fn sb_basic_append() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testBasicAppend", "()I"),
            5
        );
    }

    #[test]
    fn sb_chaining() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testChaining", "()I"),
            6
        );
    }

    #[test]
    fn sb_append_int() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testAppendInt", "()I"),
            6
        );
    }

    #[test]
    fn sb_append_long() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testAppendLong", "()I"),
            3
        );
    }

    #[test]
    fn sb_append_boolean() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testAppendBoolean", "()I"),
            4
        );
    }

    #[test]
    fn sb_append_char() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testAppendChar", "()I"),
            1
        );
    }

    #[test]
    fn sb_append_double() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testAppendDouble", "()I"),
            4
        );
    }

    #[test]
    fn sb_append_float() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testAppendFloat", "()I"),
            3
        );
    }

    #[test]
    fn sb_init_with_string() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testInitWithString", "()I"),
            8
        );
    }

    #[test]
    fn sb_length() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testLength", "()I"),
            3
        );
    }

    #[test]
    fn sb_loop_build() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testLoopBuild", "()I"),
            5
        );
    }

    #[test]
    fn sb_append_string_object() {
        assert_eq!(
            run_bootstrap_int("StringBuilderTest.class", "testAppendString", "()I"),
            5
        );
    }

    // ---- Character tests ----

    #[test]
    fn char_is_digit() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsDigit", "()I"),
            1
        );
    }

    #[test]
    fn char_is_digit_false() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsDigitFalse", "()I"),
            1
        );
    }

    #[test]
    fn char_is_letter() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsLetter", "()I"),
            1
        );
    }

    #[test]
    fn char_is_letter_false() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsLetterFalse", "()I"),
            1
        );
    }

    #[test]
    fn char_is_whitespace() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsWhitespace", "()I"),
            1
        );
    }

    #[test]
    fn char_is_uppercase() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsUpperCase", "()I"),
            1
        );
    }

    #[test]
    fn char_is_lowercase() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsLowerCase", "()I"),
            1
        );
    }

    #[test]
    fn char_to_uppercase() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testToUpperCase", "()I"),
            65
        );
    }

    #[test]
    fn char_to_lowercase() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testToLowerCase", "()I"),
            97
        );
    }

    #[test]
    fn char_is_letter_or_digit() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testIsLetterOrDigit", "()I"),
            1
        );
    }

    #[test]
    fn char_valueof_and_charvalue() {
        assert_eq!(
            run_bootstrap_int("CharacterTest.class", "testValueOf", "()I"),
            88
        );
    }

    #[test]
    fn arraylist_size() {
        assert_eq!(
            run_bootstrap_int("ArrayListTest.class", "testSize", "()I"),
            3
        );
    }

    #[test]
    fn arraylist_get() {
        assert_eq!(
            run_bootstrap_int("ArrayListTest.class", "testGet", "()I"),
            5
        );
    }

    #[test]
    fn arraylist_foreach_count() {
        assert_eq!(
            run_bootstrap_int("ArrayListTest.class", "testForEachCount", "()I"),
            3
        );
    }

    #[test]
    fn arraylist_foreach_sum() {
        assert_eq!(
            run_bootstrap_int("ArrayListTest.class", "testForEachSum", "()I"),
            8
        );
    }

    #[test]
    fn arraylist_empty_foreach() {
        assert_eq!(
            run_bootstrap_int("ArrayListTest.class", "testEmptyForEach", "()I"),
            0
        );
    }

    #[test]
    fn arraylist_single_element() {
        assert_eq!(
            run_bootstrap_int("ArrayListTest.class", "testSingleElement", "()I"),
            4
        );
    }

    #[test]
    fn arraylist_add_returns_true() {
        assert_eq!(
            run_bootstrap_int("ArrayListTest.class", "testAddReturnsTrue", "()I"),
            1
        );
    }

    // ---- Phase 22: String.format() integration tests ----

    #[test]
    fn format_string() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatString", "()I"),
            11
        );
    }

    #[test]
    fn format_int() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatInt", "()I"),
            2
        );
    }

    #[test]
    fn format_multiple() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatMultiple", "()I"),
            3
        );
    }

    #[test]
    fn format_double() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatDouble", "()I"),
            4
        );
    }

    #[test]
    fn format_hex() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatHex", "()I"),
            2
        );
    }

    #[test]
    fn format_percent() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatPercent", "()I"),
            4
        );
    }

    #[test]
    fn format_null() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatNull", "()I"),
            4
        );
    }

    #[test]
    fn format_sum() {
        assert_eq!(
            run_bootstrap_int("StringFormatTest.class", "testFormatSum", "()I"),
            8
        );
    }

    // ---- Phase 22 Task 2: Arrays utilities + numeric constants ----

    #[test]
    fn arrays_fill_int() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testFillInt", "()I"),
            14
        );
    }

    #[test]
    fn arrays_copyof_truncate() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testCopyOfTruncate", "()I"),
            3
        );
    }

    #[test]
    fn arrays_copyof_extend() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testCopyOfExtend", "()I"),
            0
        );
    }

    #[test]
    fn arrays_sort_int() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testSortInt", "()I"),
            19
        );
    }

    #[test]
    fn integer_max_value() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testIntegerMaxValue", "()I"),
            1
        );
    }

    #[test]
    fn integer_min_value() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testIntegerMinValue", "()I"),
            1
        );
    }

    #[test]
    fn long_max_value() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testLongMaxValue", "()I"),
            1
        );
    }

    #[test]
    fn double_max_value() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testDoubleMaxValue", "()I"),
            1
        );
    }

    #[test]
    fn double_nan() {
        assert_eq!(
            run_bootstrap_int("ArraysTest.class", "testDoubleNaN", "()I"),
            1
        );
    }

    // ---- Phase 23 Task 1: HashMap ----

    #[test]
    fn hashmap_put_and_get() {
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testPutAndGet", "()I"),
            30
        );
    }

    #[test]
    fn hashmap_size() {
        assert_eq!(run_bootstrap_int("HashMapTest.class", "testSize", "()I"), 3);
    }

    #[test]
    fn hashmap_contains_key() {
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testContainsKey", "()I"),
            1
        );
    }

    #[test]
    fn hashmap_get_missing() {
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testGetMissing", "()I"),
            1
        );
    }

    #[test]
    fn hashmap_remove() {
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testRemove", "()I"),
            1
        );
    }

    #[test]
    fn hashmap_is_empty() {
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testIsEmpty", "()I"),
            1
        );
    }

    #[test]
    fn hashmap_overwrite() {
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testOverwrite", "()I"),
            1
        );
    }

    #[test]
    fn hashmap_get_or_default() {
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testGetOrDefault", "()I"),
            106
        );
    }

    #[test]
    fn hashmap_overwrite_value_returns_new_value() {
        // kills 9488: fields[i+1]=val → fields[i]=val; get after overwrite returns null
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testOverwriteValue", "()I"),
            99
        );
    }

    #[test]
    fn hashmap_update_second_key_returns_new_value() {
        // kills 9491: i+=2 → i*=2; second key update is missed
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testUpdateSecondKeyValue", "()I"),
            99
        );
    }

    #[test]
    fn hashmap_remove_then_not_contains() {
        // kills 9586: truncate(len-2) → truncate(len+2); key persists after remove
        assert_eq!(
            run_bootstrap_int("HashMapTest.class", "testRemoveAndContains", "()I"),
            0
        );
    }

    #[test]
    fn hashset_add_and_contains() {
        assert_eq!(
            run_bootstrap_int("HashSetTest.class", "testAddAndContains", "()I"),
            1
        );
    }

    #[test]
    fn hashset_size() {
        assert_eq!(run_bootstrap_int("HashSetTest.class", "testSize", "()I"), 3);
    }

    #[test]
    fn hashset_no_duplicates() {
        assert_eq!(
            run_bootstrap_int("HashSetTest.class", "testNoDuplicates", "()I"),
            2
        );
    }

    #[test]
    fn hashset_remove() {
        assert_eq!(
            run_bootstrap_int("HashSetTest.class", "testRemove", "()I"),
            1
        );
    }

    #[test]
    fn hashset_is_empty() {
        assert_eq!(
            run_bootstrap_int("HashSetTest.class", "testIsEmpty", "()I"),
            1
        );
    }

    #[test]
    fn hashset_add_returns_false() {
        assert_eq!(
            run_bootstrap_int("HashSetTest.class", "testAddReturnsFalse", "()I"),
            1
        );
    }

    #[test]
    fn frame_pool_does_not_change_fib_result() {
        // Regression guard: pool reuse must not corrupt frame state.
        // fib(25) = 75025 — stale locals between pool reuses would produce wrong answer.
        let result = run_bootstrap_int("BenchmarkSuite.class", "benchFib", "()I");
        assert_eq!(result, 75025);
    }

    #[test]
    fn dispatch_cache_fib_correctness() {
        let result = run_bootstrap_int("BenchmarkSuite.class", "benchFib", "()I");
        assert_eq!(result, 75025);
    }

    #[test]
    fn dispatch_cache_invokestatic_multiple_methods() {
        let sum = run_bootstrap_int("BenchmarkSuite.class", "benchSum", "()I");
        let fib = run_bootstrap_int("BenchmarkSuite.class", "benchFib", "()I");
        assert_eq!(fib, 75025);
        // benchSum overflows i32: sum(0..499999) = 124999750000 → wraps to 445698416
        assert_eq!(sum, 445_698_416_i32);
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_bytecode_cost_counts_iadd() {
        // benchSum adds integers in a loop 0..500_000 (500_000 iadd ops).
        let (result, registry) = run_fixture("BenchmarkSuite.class", "benchSum", "()I");
        assert_eq!(result, Some(Slot::Int(445_698_416)));
        let stat = &registry.telemetry.bytecode_cost.by_opcode["iadd"];
        assert_eq!(stat.count, 500_000);
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_object_lineage_records_allocations() {
        // ForEachTest creates an ArrayList and adds elements.
        let (_, registry) = run_fixture("ForEachTest.class", "main", "([Ljava/lang/String;)V");
        // At least one allocation site must exist (the ArrayList constructor).
        assert!(!registry.telemetry.object_lineage.sites.is_empty());
        // Verify some allocation is attributed to ForEachTest class.
        let has_foreach_alloc = registry
            .telemetry
            .object_lineage
            .sites
            .keys()
            .any(|(cls, _, _)| cls == "ForEachTest");
        assert!(
            has_foreach_alloc,
            "expected at least one allocation from ForEachTest"
        );
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_native_boundary_records_println() {
        let (_, registry) = run_fixture("ForEachTest.class", "main", "([Ljava/lang/String;)V");
        // ForEachTest calls System.out.println which dispatches through println native.
        let stat = registry
            .telemetry
            .native_boundary
            .by_method
            .iter()
            .find(|((_, name), _)| name.contains("println"));
        assert!(
            stat.is_some(),
            "expected println to be recorded in native_boundary"
        );
        let (_, s) = stat.unwrap();
        assert!(s.calls > 0);
        assert_eq!(s.errors, 0);
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_exception_flow_records_throw_and_catch() {
        let (result, registry) = run_fixture("ExceptionTest.class", "throwAndCatch", "()I");
        assert_eq!(result, Some(Slot::Int(42)));
        let events = &registry.telemetry.exception_flow.events;
        assert_eq!(events.len(), 1);
        assert!(events[0].exception_class.contains("RuntimeException"));
        assert!(events[0].catch_site.is_some(), "exception should be caught");
    }

    #[cfg(feature = "telemetry")]
    #[test]
    #[allow(clippy::len_zero)]
    fn telemetry_exception_flow_rethrow_caught() {
        let (result, registry) = run_fixture("ExceptionTest.class", "rethrow", "()I");
        assert_eq!(result, Some(Slot::Int(99)));
        let events = &registry.telemetry.exception_flow.events;
        // Two throw events: inner throw + rethrow
        assert!(!events.is_empty());
        assert!(events.iter().all(|e| e.catch_site.is_some()));
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_class_init_dag_records_clinit() {
        // ClinitTest has a static initializer that sets VALUE = 42.
        // Calling getValue() via invokestatic triggers ensure_initialized → <clinit>.
        let (result, registry) = run_fixture("ClinitTest.class", "getValue", "()I");
        assert_eq!(result, Some(Slot::Int(42)));
        let events = &registry.telemetry.class_init_dag.events;
        assert!(
            !events.is_empty(),
            "expected at least one class_init_dag event"
        );
        let ev = events
            .iter()
            .find(|e| e.class == "ClinitTest")
            .expect("expected ClinitTest clinit event");
        assert!(ev.duration_ns > 0, "clinit duration should be positive");
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_dispatch_resolution_records_virtual_calls() {
        // ForEachTest calls invokevirtual on ArrayList (add) and invokeinterface
        // for the for-each iterator protocol (iterator, hasNext, next).
        let (_, registry) = run_fixture("ForEachTest.class", "main", "([Ljava/lang/String;)V");
        let dr = &registry.telemetry.dispatch_resolution;
        // At least some virtual/interface dispatch sites must have been recorded.
        assert!(
            !dr.by_site.is_empty(),
            "dispatch_resolution should have entries after ForEachTest"
        );
        // Every recorded site must have at least one call.
        for ((cls, cp), stat) in &dr.by_site {
            assert!(stat.calls > 0, "site {cls}[cp{cp}] should have calls > 0");
        }
    }

    // ---- Phase 24: GC stress tests ----

    #[test]
    fn gc_reclaims_short_lived_objects() {
        // GcStressTest allocates 2000 int[4] arrays in a loop.
        // Verify correct output sum = 0+1+...+1999 = 1999000.
        let result = run_bootstrap_int("GcStressTest.class", "run", "()I");
        assert_eq!(result, 1_999_000);
    }

    #[test]
    fn gc_keeps_heap_bounded() {
        let loader = fixtures_loader();
        let bytes = loader.find_class("GcStressTest").unwrap();
        let cf = duke_classfile::parse(&bytes).unwrap();
        let ctx = build_class_context(&cf);
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let mut stdout = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut stdout,
            "GcStressTest",
            "run",
            "()I",
            &[],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1_999_000)));
        // 2000 arrays allocated; GC should have reclaimed most.
        // bootstrap_stdlib pre-populates ~200 permanent live objects (synthetic classes,
        // interned strings, static fields). After GC fires, the 2000 short-lived arrays
        // are collected, so total live count is dominated by bootstrap objects.
        // Without GC the heap would grow to 2000+ objects; with GC it stays bounded.
        let live = heap.len();
        assert!(
            live < 500,
            "heap has {live} live objects — GC may not have fired"
        );
    }

    #[test]
    fn gc_generational_stress_test() {
        let ctx = load_class_context("GcGenerationalStressTest.class");
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // Small young_capacity to force frequent minor GCs.
        heap.young_capacity = 32;

        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 1);
        heap.get_mut(arr_ref).unwrap().fields[0] = duke_runtime::Slot::Reference(None);

        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            &entry_class,
            "main",
            "([Ljava/lang/String;)V",
            &[duke_runtime::Slot::Reference(Some(arr_ref))],
        );
        assert!(
            result.is_ok(),
            "generational GC stress test failed: {result:?}"
        );
        let output = String::from_utf8(out).unwrap();
        // Verify long-lived objects survived all minor GCs.
        for i in 0..10 {
            assert!(
                output.contains(&format!("Survivor-{i}")),
                "long-lived object Survivor-{i} missing from output:\n{output}"
            );
        }
        // sum of "tmp-N".length() for N in 0..5000 = 38890
        assert!(
            output.contains("38890"),
            "expected sum 38890 in output:\n{output}"
        );
    }

    #[test]
    fn callback_handler_is_dispatched_with_invoke_fn() {
        use std::sync::atomic::AtomicBool;
        static CALLED: AtomicBool = AtomicBool::new(false);

        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // Simple helper native: returns 42.
        registry.natives_mut().register(
            "duke/test/Helper",
            "answer",
            "()I",
            |_args, _heap, _out, _control| Ok(Some(Slot::Int(42))),
        );

        // Callback native: invokes the helper and returns its result.
        registry.natives_mut().register_callback(
            "duke/test/Caller",
            "call",
            "()I",
            |_args, heap, output, _control, ops| {
                CALLED.store(true, std::sync::atomic::Ordering::SeqCst);
                ops.invoke(heap, output, "duke/test/Helper", "answer", "()I", vec![])
            },
        );

        let loader = duke_loader::DirectoryLoader::new(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "duke/test/Caller",
            "call",
            "()I",
            &[],
        );
        assert!(result.is_ok(), "callback dispatch failed: {result:?}");
        assert_eq!(result.unwrap(), Some(Slot::Int(42)));
        assert!(
            CALLED.load(std::sync::atomic::Ordering::SeqCst),
            "callback handler was never invoked"
        );
    }

    // ---- Bytecode-level Callback dispatch tests (Sites 2-4) ----
    //
    // These tests verify that `HandlerKind::Callback` handlers fire when the
    // call site is reached via *bytecode* (invokestatic / invokevirtual /
    // invokeinterface), not just via the top-level fast-path.
    //
    // Pattern: bootstrap stdlib (so the fixture class can run), then
    // *override* one specific native with a Callback handler, run the fixture
    // bytecode, and assert both the result and the CALLED flag.

    /// Site 2 — invokestatic Callback arm.
    ///
    /// `ParseArgs.parseInt()` bytecode contains:
    ///   `invokestatic java/lang/Integer.parseInt:(Ljava/lang/String;)I`
    /// We override that registration with a Callback handler that delegates to
    /// `invoke`, proving the arm wires the closure correctly end-to-end.
    #[test]
    fn callback_fires_via_invokestatic_bytecode() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);

        let ctx = load_class_context("ParseArgs.class");
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // Override the Simple Integer.parseInt with a Callback that records
        // invocation and delegates via `invoke` back to the (already-registered)
        // helper that bootstrap_stdlib set up as a Simple handler on
        // "java/lang/Integer"/"parseInt".
        // Because we overwrite the key the Simple handler is gone — we compute
        // the parse directly inside the callback instead.
        registry.natives_mut().register_callback(
            "java/lang/Integer",
            "parseInt",
            "(Ljava/lang/String;)I",
            |args, heap, _output, _control, _invoke| {
                CALLED.store(true, Ordering::SeqCst);
                // args[0] is the String reference; extract its string_value.
                let s = match &args[0] {
                    Slot::Reference(Some(r)) => heap
                        .get(*r)
                        .ok()
                        .and_then(|o| o.string_value.clone())
                        .unwrap_or_default(),
                    _ => return Err(VmError::NullPointerException),
                };
                let n: i32 = s.parse().map_err(|_| VmError::NullPointerException)?;
                Ok(Some(Slot::Int(n)))
            },
        );

        let loader = fixtures_loader();
        // Build String[] = ["123"] for ParseArgs.parseInt
        let s_ref = heap.allocate_string("123".to_string());
        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 1);
        heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(s_ref));
        let mut out: Vec<u8> = Vec::new();

        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "parseInt",
            "([Ljava/lang/String;)I",
            &[Slot::Reference(Some(arr_ref))],
        );
        assert!(
            result.is_ok(),
            "invokestatic Callback dispatch failed: {result:?}"
        );
        assert_eq!(result.unwrap(), Some(Slot::Int(123)));
        assert!(
            CALLED.load(Ordering::SeqCst),
            "Callback handler was never invoked via invokestatic bytecode"
        );
    }

    /// Site 3 — invokevirtual Callback arm.
    ///
    /// `ParseArgs.valueOf()` bytecode contains:
    ///   `invokevirtual java/lang/Integer.intValue:()I`
    /// We override that registration with a Callback handler.
    #[test]
    fn callback_fires_via_invokevirtual_bytecode() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);

        let ctx = load_class_context("ParseArgs.class");
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // Override Integer.intValue with a Callback.
        // The Integer heap object stores the boxed int in fields[0].
        registry.natives_mut().register_callback(
            "java/lang/Integer",
            "intValue",
            "()I",
            |args, heap, _output, _control, _invoke| {
                CALLED.store(true, Ordering::SeqCst);
                // args[0] is `this` (the Integer object); fields[0] holds the int.
                let Slot::Reference(Some(r)) = &args[0] else {
                    return Err(VmError::NullPointerException);
                };
                let val = heap.get(*r)?.fields[0];
                Ok(Some(val))
            },
        );

        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();

        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ParseArgs",
            "valueOf",
            "()I",
            &[],
        );
        assert!(
            result.is_ok(),
            "invokevirtual Callback dispatch failed: {result:?}"
        );
        assert_eq!(result.unwrap(), Some(Slot::Int(42)));
        assert!(
            CALLED.load(Ordering::SeqCst),
            "Callback handler was never invoked via invokevirtual bytecode"
        );
    }

    /// Site 4 — invokeinterface Callback arm.
    ///
    /// `ArrayListTest.testForEachCount()` bytecode uses:
    ///   `invokeinterface java/util/Iterator.hasNext:()Z`
    /// dispatched on the actual runtime class `duke/util/ArrayListIterator`.
    /// We override `duke/util/ArrayListIterator.hasNext` with a Callback that
    /// immediately returns false (0), making the for-each body not execute and
    /// the count stay at 0.  This verifies the invokeinterface Callback arm
    /// fires.
    #[test]
    fn callback_fires_via_invokeinterface_bytecode() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);

        let ctx = load_class_context("ArrayListTest.class");
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // Override ArrayListIterator.hasNext with a Callback that records
        // invocation and immediately signals "no more elements" (returns false).
        registry.natives_mut().register_callback(
            "duke/util/ArrayListIterator",
            "hasNext",
            "()Z",
            |_args, _heap, _output, _control, _invoke| {
                CALLED.store(true, Ordering::SeqCst);
                Ok(Some(Slot::Int(0))) // false — loop body never runs
            },
        );

        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();

        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "ArrayListTest",
            "testForEachCount",
            "()I",
            &[],
        );
        assert!(
            result.is_ok(),
            "invokeinterface Callback dispatch failed: {result:?}"
        );
        // hasNext always returns false → loop body never runs → count = 0.
        assert_eq!(result.unwrap(), Some(Slot::Int(0)));
        assert!(
            CALLED.load(Ordering::SeqCst),
            "Callback handler was never invoked via invokeinterface bytecode"
        );
    }

    /// Site 5 — lambda SAM virtual/interface native-fallback Callback arm.
    ///
    /// `LambdaCallbackTest.capturedLengthViaMethodRef("hello")` compiles to:
    ///
    ///   invokedynamic … get:(Ljava/lang/String;)LLambdaCallbackTest$IntSupplier;
    ///   // creates `$$Lambda$0` with `impl_class`="java/lang/String",
    ///   //   `impl_method`="length", `impl_kind`=5 (`REF_invokeVirtual`),
    ///   //   `captured_count`=1 (the string "hello")
    ///   invokeinterface LambdaCallbackTest$IntSupplier.get:()I
    ///   // → lambda SAM: `impl_kind`==5, `resolve_method_in_hierarchy` returns None
    ///   //   (String has no bytecode methods in Duke), so falls to Site 5:
    ///   //   `registry.natives_mut().get_kind`("java/lang/String", "length", "()I")
    ///
    /// We override `String.length` with a Callback handler to prove the arm fires.
    #[test]
    fn callback_fires_via_lambda_sam_fallback() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);

        let ctx = load_class_context("LambdaCallbackTest.class");
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // javac emits `dup; invokestatic Objects.requireNonNull; pop` for
        // captured instance method references.  Register a ClassContext and a
        // passthrough native so the invokestatic dispatch doesn't fail.
        registry.register(ClassContext {
            class_name: "java/util/Objects".to_string(),
            super_class: Some("java/lang/Object".to_string()),
            constant_pool: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            static_fields: Vec::new(),
            instance_field_count: 0,
            interfaces: Vec::new(),
            bootstrap_methods: Vec::new(),
        });
        registry.natives_mut().register(
            "java/util/Objects",
            "requireNonNull",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            |args, _heap, _out, _control| Ok(Some(args[0])),
        );

        // Override String.length with a Callback.  This replaces the Simple
        // handler that bootstrap_stdlib registered, so the lambda SAM fallback
        // (Site 5) must route through the Callback arm to fire at all.
        registry.natives_mut().register_callback(
            "java/lang/String",
            "length",
            "()I",
            |args, heap, _output, _control, _invoke| {
                CALLED.store(true, Ordering::SeqCst);
                // args[0] is `this` (the captured String reference).
                let Slot::Reference(Some(r)) = &args[0] else {
                    return Err(VmError::NullPointerException);
                };
                let len = i32::try_from(heap.get(*r)?.string_value.as_deref().unwrap_or("").len())
                    .unwrap_or(i32::MAX);
                Ok(Some(Slot::Int(len)))
            },
        );

        let loader = fixtures_loader();
        let s_ref = heap.allocate_string("hello".to_string());
        let mut out: Vec<u8> = Vec::new();

        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "LambdaCallbackTest",
            "capturedLengthViaMethodRef",
            "(Ljava/lang/String;)I",
            &[Slot::Reference(Some(s_ref))],
        );
        assert!(
            result.is_ok(),
            "lambda SAM Callback dispatch failed: {result:?}"
        );
        assert_eq!(result.unwrap(), Some(Slot::Int(5)));
        assert!(
            CALLED.load(Ordering::SeqCst),
            "Callback handler was never invoked via lambda SAM fallback (Site 5)"
        );
    }

    #[test]
    fn integer_compare_to_less_returns_negative() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let a = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(a).unwrap().fields[0] = Slot::Int(3);
        let b = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(b).unwrap().fields[0] = Slot::Int(5);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/lang/Integer",
            "compareTo",
            "(Ljava/lang/Object;)I",
            &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(-1)));
    }

    #[test]
    fn integer_compare_to_equal_returns_zero() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let a = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(a).unwrap().fields[0] = Slot::Int(7);
        let b = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(b).unwrap().fields[0] = Slot::Int(7);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/lang/Integer",
            "compareTo",
            "(Ljava/lang/Object;)I",
            &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(0)));
    }

    #[test]
    fn string_compare_to_apple_less_than_banana() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let a = heap.allocate_string("apple".to_string());
        let b = heap.allocate_string("banana".to_string());
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/lang/String",
            "compareTo",
            "(Ljava/lang/Object;)I",
            &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
        )
        .unwrap();
        match result {
            Some(Slot::Int(n)) => assert!(n < 0, "apple < banana: expected negative, got {n}"),
            other => panic!("expected Int, got {other:?}"),
        }
    }

    #[test]
    fn integer_compare_to_greater_returns_positive() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let a = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(a).unwrap().fields[0] = Slot::Int(9);
        let b = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(b).unwrap().fields[0] = Slot::Int(3);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/lang/Integer",
            "compareTo",
            "(Ljava/lang/Object;)I",
            &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn long_compare_to_less_returns_negative() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let a = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(a).unwrap().fields[0] = Slot::Long(100);
        let b = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(b).unwrap().fields[0] = Slot::Long(200);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/lang/Long",
            "compareTo",
            "(Ljava/lang/Object;)I",
            &[Slot::Reference(Some(a)), Slot::Reference(Some(b))],
        )
        .unwrap();
        assert_eq!(result, Some(Slot::Int(-1)));
    }

    #[test]
    fn double_compare_to_nan_is_greatest() {
        // Java spec: NaN > any value including POSITIVE_INFINITY.
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();

        // Case 1: NaN > +∞ → result should be 1.
        let nan_ref = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(nan_ref).unwrap().fields[0] = Slot::Double(f64::NAN);
        let inf_ref = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(inf_ref).unwrap().fields[0] = Slot::Double(f64::INFINITY);
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/lang/Double",
            "compareTo",
            "(Ljava/lang/Object;)I",
            &[
                Slot::Reference(Some(nan_ref)),
                Slot::Reference(Some(inf_ref)),
            ],
        )
        .unwrap();
        assert_eq!(
            result,
            Some(Slot::Int(1)),
            "NaN.compareTo(+Inf) should be 1 (NaN is greatest)"
        );

        // Case 2: 1.0 < NaN → result should be -1.
        let one_ref = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(one_ref).unwrap().fields[0] = Slot::Double(1.0);
        let nan_ref2 = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(nan_ref2).unwrap().fields[0] = Slot::Double(f64::NAN);
        let mut out2: Vec<u8> = Vec::new();
        let result2 = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out2,
            "java/lang/Double",
            "compareTo",
            "(Ljava/lang/Object;)I",
            &[
                Slot::Reference(Some(one_ref)),
                Slot::Reference(Some(nan_ref2)),
            ],
        )
        .unwrap();
        assert_eq!(
            result2,
            Some(Slot::Int(-1)),
            "1.0.compareTo(NaN) should be -1 (NaN is greatest)"
        );
    }

    // ---- Phase 26 Task 4: ArrayList.sort(Comparator) via CallbackNativeHandler ----

    #[test]
    fn array_list_sort_integers_via_callback() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // Build ArrayList [Integer(3), Integer(1), Integer(4)]
        let list = heap.allocate("java/util/ArrayList".to_string(), 4);
        heap.get_mut(list).unwrap().fields[0] = Slot::Int(3); // size
        let make_int = |heap: &mut duke_gc::Heap, n: i32| -> u64 {
            let r = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(r).unwrap().fields[0] = Slot::Int(n);
            r
        };
        let i3 = make_int(&mut heap, 3);
        let i1 = make_int(&mut heap, 1);
        let i4 = make_int(&mut heap, 4);
        heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(i3));
        heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(i1));
        heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(i4));

        let loader = duke_loader::DirectoryLoader::new(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/ArrayList",
            "sort",
            "(Ljava/util/Comparator;)V",
            &[Slot::Reference(Some(list)), Slot::Reference(None)], // null Comparator
        )
        .unwrap();

        // After sort: fields[1..=3] = Integer(1), Integer(3), Integer(4)
        let val = |heap: &duke_gc::Heap, s: &Slot| -> i32 {
            match s {
                Slot::Reference(Some(r)) => match heap.get(*r).unwrap().fields.first() {
                    Some(Slot::Int(n)) => *n,
                    _ => -1,
                },
                _ => -1,
            }
        };
        let f = |i: usize| heap.get(list).unwrap().fields[i];
        assert_eq!(val(&heap, &f(1)), 1);
        assert_eq!(val(&heap, &f(2)), 3);
        assert_eq!(val(&heap, &f(3)), 4);
    }

    #[test]
    fn array_list_sort_strings_via_callback() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        let list = heap.allocate("java/util/ArrayList".to_string(), 4);
        heap.get_mut(list).unwrap().fields[0] = Slot::Int(3);
        let sb = heap.allocate_string("banana".to_string());
        let sa = heap.allocate_string("apple".to_string());
        let sc = heap.allocate_string("cherry".to_string());
        heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(sb));
        heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(sa));
        heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(sc));

        let loader = duke_loader::DirectoryLoader::new(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/ArrayList",
            "sort",
            "(Ljava/util/Comparator;)V",
            &[Slot::Reference(Some(list)), Slot::Reference(None)],
        )
        .unwrap();

        let str_val = |heap: &duke_gc::Heap, s: &Slot| -> String {
            match s {
                Slot::Reference(Some(r)) => heap
                    .get(*r)
                    .unwrap()
                    .string_value
                    .clone()
                    .unwrap_or_default(),
                _ => String::new(),
            }
        };
        let f = |i: usize| heap.get(list).unwrap().fields[i];
        assert_eq!(str_val(&heap, &f(1)), "apple");
        assert_eq!(str_val(&heap, &f(2)), "banana");
        assert_eq!(str_val(&heap, &f(3)), "cherry");
    }

    // Fix 6: boundary tests for array_list_sort

    #[test]
    fn array_list_sort_empty_list_is_noop() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // ArrayList with size=0 — allocate just the size field slot.
        let list = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(list).unwrap().fields[0] = Slot::Int(0);

        let loader = duke_loader::DirectoryLoader::new(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/ArrayList",
            "sort",
            "(Ljava/util/Comparator;)V",
            &[Slot::Reference(Some(list)), Slot::Reference(None)],
        );
        assert!(result.is_ok(), "empty sort should not error: {result:?}");
        assert_eq!(result.unwrap(), None);
        // Size field still 0.
        assert_eq!(heap.get(list).unwrap().fields[0], Slot::Int(0));
    }

    #[test]
    fn array_list_sort_single_element_is_noop() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        let list = heap.allocate("java/util/ArrayList".to_string(), 2);
        heap.get_mut(list).unwrap().fields[0] = Slot::Int(1);
        let elem = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(elem).unwrap().fields[0] = Slot::Int(42);
        heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(elem));

        let loader = duke_loader::DirectoryLoader::new(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/ArrayList",
            "sort",
            "(Ljava/util/Comparator;)V",
            &[Slot::Reference(Some(list)), Slot::Reference(None)],
        )
        .unwrap();

        // Single element unchanged.
        match &heap.get(list).unwrap().fields[1] {
            Slot::Reference(Some(r)) => {
                let r = *r;
                assert_eq!(heap.get(r).unwrap().fields[0], Slot::Int(42));
            }
            other => panic!("unexpected slot: {other:?}"),
        }
    }

    #[test]
    fn array_list_sort_already_sorted_unchanged() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        let list = heap.allocate("java/util/ArrayList".to_string(), 4);
        heap.get_mut(list).unwrap().fields[0] = Slot::Int(3);
        let make_int = |heap: &mut duke_gc::Heap, n: i32| -> u64 {
            let r = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(r).unwrap().fields[0] = Slot::Int(n);
            r
        };
        let i1 = make_int(&mut heap, 1);
        let i2 = make_int(&mut heap, 2);
        let i3 = make_int(&mut heap, 3);
        heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(i1));
        heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(i2));
        heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(i3));

        let loader = duke_loader::DirectoryLoader::new(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/ArrayList",
            "sort",
            "(Ljava/util/Comparator;)V",
            &[Slot::Reference(Some(list)), Slot::Reference(None)],
        )
        .unwrap();

        let int_val = |heap: &duke_gc::Heap, i: usize| -> i32 {
            match heap.get(list).unwrap().fields[i] {
                Slot::Reference(Some(r)) => match heap.get(r).unwrap().fields[0] {
                    Slot::Int(n) => n,
                    _ => -1,
                },
                _ => -1,
            }
        };
        assert_eq!(int_val(&heap, 1), 1);
        assert_eq!(int_val(&heap, 2), 2);
        assert_eq!(int_val(&heap, 3), 3);
    }

    #[test]
    fn array_list_sort_duplicates() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // [3, 1, 1, 2] → [1, 1, 2, 3]
        let list = heap.allocate("java/util/ArrayList".to_string(), 5);
        heap.get_mut(list).unwrap().fields[0] = Slot::Int(4);
        let make_int = |heap: &mut duke_gc::Heap, n: i32| -> u64 {
            let r = heap.allocate("java/lang/Integer".to_string(), 1);
            heap.get_mut(r).unwrap().fields[0] = Slot::Int(n);
            r
        };
        let r3 = make_int(&mut heap, 3);
        let r1a = make_int(&mut heap, 1);
        let r1b = make_int(&mut heap, 1);
        let r2 = make_int(&mut heap, 2);
        heap.get_mut(list).unwrap().fields[1] = Slot::Reference(Some(r3));
        heap.get_mut(list).unwrap().fields[2] = Slot::Reference(Some(r1a));
        heap.get_mut(list).unwrap().fields[3] = Slot::Reference(Some(r1b));
        heap.get_mut(list).unwrap().fields[4] = Slot::Reference(Some(r2));

        let loader = duke_loader::DirectoryLoader::new(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures"),
        );
        let mut out: Vec<u8> = Vec::new();
        execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/ArrayList",
            "sort",
            "(Ljava/util/Comparator;)V",
            &[Slot::Reference(Some(list)), Slot::Reference(None)],
        )
        .unwrap();

        let int_val = |heap: &duke_gc::Heap, i: usize| -> i32 {
            match heap.get(list).unwrap().fields[i] {
                Slot::Reference(Some(r)) => match heap.get(r).unwrap().fields[0] {
                    Slot::Int(n) => n,
                    _ => -1,
                },
                _ => -1,
            }
        };
        assert_eq!(int_val(&heap, 1), 1);
        assert_eq!(int_val(&heap, 2), 1);
        assert_eq!(int_val(&heap, 3), 2);
        assert_eq!(int_val(&heap, 4), 3);
    }

    // ---- Phase 26 Task 5: CollectionsSortTest end-to-end integration test ----

    #[test]
    fn collections_sort_end_to_end() {
        let ctx = load_class_context("CollectionsSortTest.class");
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        let loader = fixtures_loader();
        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 1);
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            &entry_class,
            "main",
            "([Ljava/lang/String;)V",
            &[Slot::Reference(Some(arr_ref))],
        );
        assert!(result.is_ok(), "CollectionsSortTest failed: {result:?}");
        let output = String::from_utf8(out).unwrap();
        // Integer sort: 1 1 3 4 5 (one per line)
        assert!(
            output.contains("1\n1\n3\n4\n5"),
            "integer sort wrong:\n{output}"
        );
        // String sort: apple banana cherry (one per line)
        assert!(
            output.contains("apple\nbanana\ncherry"),
            "string sort wrong:\n{output}"
        );
    }

    #[test]
    fn collections_sort_null_list_raises_npe() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/Collections",
            "sort",
            "(Ljava/util/List;)V",
            &[Slot::Reference(None)],
        );
        assert!(
            matches!(result, Err(VmError::NullPointerException)),
            "expected NullPointerException, got {result:?}"
        );
    }

    #[test]
    fn collections_sort_empty_list_is_noop() {
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        // Empty ArrayList: size=0
        let list = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(list).unwrap().fields[0] = Slot::Int(0);
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "java/util/Collections",
            "sort",
            "(Ljava/util/List;)V",
            &[Slot::Reference(Some(list))],
        );
        assert!(result.is_ok(), "empty list sort failed: {result:?}");
    }

    // --- Phase 27: try-with-resources ---

    #[test]
    fn try_with_resources_simple_value() {
        assert_eq!(
            run_bootstrap_int("TryWithResources.class", "simpleValue", "()I"),
            42
        );
    }

    #[test]
    fn try_with_resources_closed_on_success() {
        assert_eq!(
            run_bootstrap_int("TryWithResources.class", "closedOnSuccess", "()I"),
            1
        );
    }

    #[test]
    fn try_with_resources_closed_on_exception() {
        assert_eq!(
            run_bootstrap_int("TryWithResources.class", "closedOnException", "()I"),
            1
        );
    }

    #[test]
    fn try_with_resources_nested_closed() {
        assert_eq!(
            run_bootstrap_int("TryWithResources.class", "nestedClosed", "()I"),
            2
        );
    }

    // ---- Phase 28: File I/O metadata ----

    #[test]
    fn file_io_metadata_reports_file_and_directory_kinds() {
        let (root, _cleanup) = make_temp_root("duke-file-io");

        let file_path = root.join("sample.txt");
        let dir_path = root.join("nested");
        let missing_path = root.join("missing.txt");
        std::fs::write(&file_path, b"abc").expect("write temp file");
        std::fs::create_dir_all(&dir_path).expect("create temp dir");

        let result = run_bootstrap_with_string_args(
            "FileIoTest.class",
            "inspectKinds",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
            &[
                file_path.to_string_lossy().into_owned(),
                dir_path.to_string_lossy().into_owned(),
                missing_path.to_string_lossy().into_owned(),
            ],
        );

        assert!(
            result.is_ok(),
            "expected File metadata support; current Duke failed with {result:?}"
        );
        let Some(Slot::Int(mask)) = result.unwrap() else {
            panic!("expected int bitmask result");
        };
        assert_ne!(mask & 1, 0, "existing file should report exists()");
        assert_ne!(mask & 2, 0, "existing file should report isFile()");
        assert_eq!(mask & 4, 0, "existing file should not report isDirectory()");
        assert_ne!(mask & 8, 0, "existing directory should report exists()");
        assert_eq!(
            mask & 16,
            0,
            "existing directory should not report isFile()"
        );
        assert_ne!(
            mask & 32,
            0,
            "existing directory should report isDirectory()"
        );
        assert_eq!(mask & 64, 0, "missing path should not report exists()");
        assert_eq!(mask & 128, 0, "missing path should not report isFile()");
        assert_eq!(
            mask & 256,
            0,
            "missing path should not report isDirectory()"
        );
    }

    #[test]
    fn file_io_reads_all_bytes_and_sums_them() {
        let (root, _cleanup) = make_temp_root("duke-file-io-read");
        let input_path = root.join("bytes.bin");
        std::fs::write(&input_path, [1_u8, 2, 3, 4]).expect("write input file");

        let result = run_bootstrap_with_string_args(
            "FileIoTest.class",
            "readAllAndSum",
            "(Ljava/lang/String;)I",
            &[input_path.to_string_lossy().into_owned()],
        );

        assert!(
            result.is_ok(),
            "expected FileInputStream read support; current Duke failed with {result:?}"
        );
        assert_eq!(
            result.unwrap(),
            Some(Slot::Int(10)),
            "readAllAndSum should return the sum of all input bytes"
        );
    }

    #[test]
    fn file_io_copies_bytes_via_try_with_resources() {
        let (root, _cleanup) = make_temp_root("duke-file-io-copy");
        let input_path = root.join("input.bin");
        let output_path = root.join("output.bin");
        let input_bytes = [9_u8, 8, 7, 6];
        std::fs::write(&input_path, input_bytes).expect("write input file");

        let result = run_bootstrap_with_string_args(
            "FileIoTest.class",
            "copyAndCount",
            "(Ljava/lang/String;Ljava/lang/String;)I",
            &[
                input_path.to_string_lossy().into_owned(),
                output_path.to_string_lossy().into_owned(),
            ],
        );

        assert!(
            result.is_ok(),
            "expected FileOutputStream write support; current Duke failed with {result:?}"
        );
        assert_eq!(
            result.unwrap(),
            Some(Slot::Int(4)),
            "copyAndCount should report the number of copied bytes"
        );
        assert_eq!(
            std::fs::read(&output_path).expect("read output file"),
            input_bytes,
            "copied output bytes should match the input bytes"
        );
    }

    #[test]
    fn file_io_copies_bytes_with_block_read_and_write() {
        let (root, _cleanup) = make_temp_root("duke-file-io-buffer");
        let input_path = root.join("input.bin");
        let output_path = root.join("output.bin");
        let input_bytes = [5_u8, 4, 3, 2];
        std::fs::write(&input_path, input_bytes).expect("write input file");

        let result = run_bootstrap_with_string_args(
            "FileIoTest.class",
            "copyWithBuffer",
            "(Ljava/lang/String;Ljava/lang/String;)I",
            &[
                input_path.to_string_lossy().into_owned(),
                output_path.to_string_lossy().into_owned(),
            ],
        );

        assert!(
            result.is_ok(),
            "expected block File I/O support; current Duke failed with {result:?}"
        );
        assert_eq!(
            result.unwrap(),
            Some(Slot::Int(4)),
            "copyWithBuffer should report the number of buffered bytes"
        );
        assert_eq!(
            std::fs::read(&output_path).expect("read output file"),
            input_bytes,
            "block copy output bytes should match the input bytes"
        );
    }

    #[test]
    fn file_io_missing_input_raises_file_not_found() {
        let (root, _cleanup) = make_temp_root("duke-file-io-missing");
        let missing_path = root.join("missing.bin");

        let result = run_bootstrap_with_string_args(
            "FileIoTest.class",
            "missingFile",
            "(Ljava/lang/String;)I",
            &[missing_path.to_string_lossy().into_owned()],
        );

        assert_eq!(
            result.unwrap(),
            Some(Slot::Int(1)),
            "missingFile should catch FileNotFoundException"
        );
    }

    #[test]
    fn file_io_read_after_close_raises_io_exception() {
        let (root, _cleanup) = make_temp_root("duke-file-io-read-close");
        let input_path = root.join("input.bin");
        std::fs::write(&input_path, [1_u8, 2, 3]).expect("write input file");

        let result = run_bootstrap_with_string_args(
            "FileIoTest.class",
            "readAfterClose",
            "(Ljava/lang/String;)I",
            &[input_path.to_string_lossy().into_owned()],
        );

        assert_eq!(
            result.unwrap(),
            Some(Slot::Int(1)),
            "readAfterClose should catch IOException"
        );
    }

    #[test]
    fn file_io_write_after_close_raises_io_exception() {
        let (root, _cleanup) = make_temp_root("duke-file-io-write-close");
        let output_path = root.join("output.bin");

        let result = run_bootstrap_with_string_args(
            "FileIoTest.class",
            "writeAfterClose",
            "(Ljava/lang/String;)I",
            &[output_path.to_string_lossy().into_owned()],
        );

        assert_eq!(
            result.unwrap(),
            Some(Slot::Int(1)),
            "writeAfterClose should catch IOException"
        );
    }

    // ---- Phase 28: Threading ----

    #[test]
    fn threading_havoc_wait_for_all_java_threads_error_path() {
        let ctx = load_class_context("ThreadingTest.class");
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();

        registry
            .natives_mut()
            .register("java/lang/Thread", "sleep", "(J)V", |_, _, _, _| {
                Err(VmError::Unimplemented {
                    mnemonic: "Test panic simulation",
                })
            });

        let result = execute_class_to_completion(
            &mut registry,
            loader,
            &mut heap,
            &mut out,
            &entry_class,
            "spawnAndJoinTen",
            "()I",
            &[],
        );

        assert!(matches!(
            result,
            Err(VmError::Unimplemented {
                mnemonic: "Test panic simulation"
            })
        ));
    }

    #[test]
    fn threading_havoc_wait_for_all_java_threads_rust_panic_path() {
        let ctx = load_class_context("ThreadingTest.class");
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();

        registry
            .natives_mut()
            .register("java/lang/Thread", "sleep", "(J)V", |_, _, _, _| {
                panic!("Test rust panic simulation");
            });

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = execute_class_to_completion(
                &mut registry,
                loader,
                &mut heap,
                &mut out,
                &entry_class,
                "spawnAndJoinTen",
                "()I",
                &[],
            );
        }));

        assert!(result.is_err(), "Expected the panic to propagate");
        // We don't care about the specific error message as long as it propagated
        // It could be 'Test rust panic simulation' or 'PoisonError'
    }
    #[test]
    fn threading_spawn_and_join_ten_workers() {
        let start = std::time::Instant::now();
        let result = run_bootstrap_with_output("ThreadingTest.class", "spawnAndJoinTen", "()I");
        let elapsed = start.elapsed();

        assert!(
            result.is_ok(),
            "expected basic Thread/Runnable support; current Duke failed with {result:?}"
        );
        let (value, mut lines) = result.unwrap();
        lines.sort();
        assert_eq!(
            value,
            Some(Slot::Int(10)),
            "spawnAndJoinTen should return 10"
        );
        assert_eq!(lines.len(), 20, "ten workers should print two lines each");
        assert!(
            elapsed >= std::time::Duration::from_millis(40),
            "spawnAndJoinTen should observe real sleep time; elapsed={elapsed:?}"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(450),
            "spawnAndJoinTen should complete faster than serialized sleeps; elapsed={elapsed:?}"
        );
        for expected in 0..10 {
            assert!(
                lines.iter().any(|line| line == &expected.to_string()),
                "missing worker start line for {expected}: {lines:?}"
            );
            assert!(
                lines
                    .iter()
                    .any(|line| line == &(expected + 100).to_string()),
                "missing worker finish line for {}: {lines:?}",
                expected + 100
            );
            assert!(
                !lines
                    .iter()
                    .any(|line| line == &(-1000 - expected).to_string()),
                "worker {expected} reported a sleep failure: {lines:?}"
            );
        }
    }

    #[test]
    fn threading_subclass_run_method_wins_over_base_thread() {
        let result = run_bootstrap_with_output("ThreadingTest.class", "subclassRunWins", "()I");

        assert!(
            result.is_ok(),
            "expected Thread subclass support; current Duke failed with {result:?}"
        );
        let (value, lines) = result.unwrap();
        assert_eq!(value, Some(Slot::Int(1)), "subclassRunWins should return 1");
        assert!(
            lines.iter().any(|line| line == "205"),
            "expected subclass run() output to contain 205, got {lines:?}"
        );
    }

    #[test]
    fn threading_fire_and_forget_still_waits_for_workers_before_returning() {
        let result =
            run_bootstrap_with_output("ThreadingTest.class", "fireAndForgetStillFinishes", "()I");

        assert!(
            result.is_ok(),
            "expected Duke to keep the VM alive for worker threads; current Duke failed with {result:?}"
        );
        let (value, mut lines) = result.unwrap();
        lines.sort();
        assert_eq!(
            value,
            Some(Slot::Int(3)),
            "fireAndForgetStillFinishes should return 3"
        );
        assert_eq!(
            lines.len(),
            6,
            "three workers should still finish before the VM returns"
        );
        for expected in [0, 1, 2, 100, 101, 102] {
            assert!(
                lines.iter().any(|line| line == &expected.to_string()),
                "missing expected worker output {expected}: {lines:?}"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // ClassRegistry unit tests
    // ---------------------------------------------------------------------------

    #[test]
    fn class_registry_register_lambda_increments_counter() {
        let mut reg = ClassRegistry::new();
        let info = LambdaInfo {
            impl_class: "Foo".to_string(),
            impl_method: "lambda$0".to_string(),
            impl_desc: "()V".to_string(),
            impl_kind: 6,
            sam_method: "run".to_string(),
            sam_desc: "()V".to_string(),
            captured_count: 0,
        };
        let n0 = reg.register_lambda(info.clone());
        let n1 = reg.register_lambda(info);
        assert_ne!(n0, n1, "each lambda gets a distinct name");
        assert!(n0.contains('0'), "first lambda name contains '0'");
        assert!(n1.contains('1'), "second lambda name contains '1'");
    }

    #[test]
    fn class_registry_natives_returns_registry() {
        #[allow(clippy::unnecessary_wraps)]
        fn dummy(
            _args: &[Slot],
            _heap: &mut duke_gc::Heap,
            _out: &mut dyn std::io::Write,
            _control: &mut NativeControl,
        ) -> VmResult<Option<Slot>> {
            Ok(Some(Slot::Int(99)))
        }
        let mut reg = ClassRegistry::new();
        reg.natives_mut().register("C", "m", "()I", dummy);
        assert!(
            reg.natives_mut().get_kind("C", "m", "()I").is_some(),
            "natives() getter must expose registered handler"
        );
    }

    #[test]
    fn class_registry_contains_true_after_register() {
        let mut reg = ClassRegistry::new();
        let ctx = ClassContext {
            class_name: "Foo".to_string(),
            super_class: None,
            interfaces: vec![],
            constant_pool: vec![],
            methods: vec![],
            fields: vec![],
            static_fields: vec![],
            instance_field_count: 0,
            bootstrap_methods: vec![],
        };
        assert!(!reg.contains("Foo"));
        reg.register(ctx);
        assert!(reg.contains("Foo"));
    }

    #[test]
    fn class_registry_all_classes_counts_after_bootstrap() {
        let mut reg = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut reg, &mut heap);
        let count = reg.all_classes().count();
        assert!(count > 10, "bootstrap registers many classes; got {count}");
    }

    #[test]
    fn class_registry_all_classes_mut_allows_mutation() {
        let mut reg = ClassRegistry::new();
        let ctx = ClassContext {
            class_name: "Bar".to_string(),
            super_class: None,
            interfaces: vec![],
            constant_pool: vec![],
            methods: vec![],
            fields: vec![],
            static_fields: vec![Slot::Int(1)],
            instance_field_count: 0,
            bootstrap_methods: vec![],
        };
        reg.register(ctx);
        for cls in reg.all_classes_mut() {
            for slot in &mut cls.static_fields {
                if let Slot::Int(v) = slot {
                    *v = 42;
                }
            }
        }
        let bar = reg.get("Bar").unwrap();
        assert_eq!(bar.static_fields[0], Slot::Int(42));
    }

    // ---------------------------------------------------------------------------
    // heap_object_to_string
    // ---------------------------------------------------------------------------

    #[test]
    fn heap_object_to_string_integer_field() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Int(42);
        let obj = heap.get(r).unwrap().clone();
        assert_eq!(heap_object_to_string(&obj, r), "42");
    }

    #[test]
    fn heap_object_to_string_long_field() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Long(999_000_000_000_i64);
        let obj = heap.get(r).unwrap().clone();
        assert_eq!(heap_object_to_string(&obj, r), "999000000000");
    }

    #[test]
    fn heap_object_to_string_double_field() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Double(std::f64::consts::PI);
        let obj = heap.get(r).unwrap().clone();
        let s = heap_object_to_string(&obj, r);
        assert!(s.contains("3.14"), "expected '3.14' in '{s}'");
    }

    #[test]
    fn heap_object_to_string_float_field() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Float".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Float(1.5_f32);
        let obj = heap.get(r).unwrap().clone();
        assert_eq!(heap_object_to_string(&obj, r), "1.5");
    }

    #[test]
    fn heap_object_to_string_boolean_true() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Int(1);
        let obj = heap.get(r).unwrap().clone();
        assert_eq!(heap_object_to_string(&obj, r), "true");
    }

    #[test]
    fn heap_object_to_string_boolean_false() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Int(0);
        let obj = heap.get(r).unwrap().clone();
        assert_eq!(heap_object_to_string(&obj, r), "false");
    }

    #[test]
    fn heap_object_to_string_boolean_nonzero_is_true() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Boolean".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Int(7);
        let obj = heap.get(r).unwrap().clone();
        assert_eq!(heap_object_to_string(&obj, r), "true");
    }

    #[test]
    fn heap_object_to_string_character_field() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Character".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Int('A' as i32);
        let obj = heap.get(r).unwrap().clone();
        assert_eq!(heap_object_to_string(&obj, r), "A");
    }

    #[test]
    fn heap_object_to_string_string_value_takes_priority() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate_string("hello".to_string());
        let obj = heap.get(r).unwrap().clone();
        assert_eq!(heap_object_to_string(&obj, r), "hello");
    }

    #[test]
    fn heap_object_to_string_opaque_object_uses_class_at_hex() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Object".to_string(), 0);
        let obj = heap.get(r).unwrap().clone();
        let s = heap_object_to_string(&obj, r);
        assert!(
            s.starts_with("java/lang/Object@"),
            "expected 'java/lang/Object@...' but got '{s}'"
        );
    }

    // ---------------------------------------------------------------------------
    // format_java_float / format_java_double
    // ---------------------------------------------------------------------------

    #[test]
    fn format_java_float_nan() {
        assert_eq!(format_java_float(f32::NAN), "NaN");
    }

    #[test]
    fn format_java_float_positive_infinity() {
        assert_eq!(format_java_float(f32::INFINITY), "Infinity");
    }

    #[test]
    fn format_java_float_negative_infinity() {
        assert_eq!(format_java_float(f32::NEG_INFINITY), "-Infinity");
    }

    #[test]
    fn format_java_float_finite_with_decimal() {
        let s = format_java_float(std::f32::consts::PI);
        assert!(s.contains('.'), "finite float must contain '.': {s}");
    }

    #[test]
    fn format_java_float_whole_number_gets_dot_zero() {
        let s = format_java_float(2.0_f32);
        assert!(
            s.ends_with(".0") || s.contains('.'),
            "must have decimal: {s}"
        );
    }

    #[test]
    fn format_java_double_nan() {
        assert_eq!(format_java_double(f64::NAN), "NaN");
    }

    #[test]
    fn format_java_double_positive_infinity() {
        assert_eq!(format_java_double(f64::INFINITY), "Infinity");
    }

    #[test]
    fn format_java_double_negative_infinity() {
        assert_eq!(format_java_double(f64::NEG_INFINITY), "-Infinity");
    }

    #[test]
    fn format_java_double_finite_with_decimal() {
        let s = format_java_double(std::f64::consts::E);
        assert!(s.contains('.'), "finite double must contain '.': {s}");
    }

    // ---------------------------------------------------------------------------
    // native_println_string / native_print_string null arms
    // ---------------------------------------------------------------------------

    #[test]
    fn native_println_string_null_ref_prints_null() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let result = native_println_string(
            &[Slot::Reference(None), Slot::Reference(None)],
            &mut heap,
            &mut out,
        );
        assert!(result.is_ok());
        assert_eq!(String::from_utf8(out).unwrap().trim(), "null");
    }

    #[test]
    fn native_print_string_null_ref_prints_null() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let result = native_print_string(
            &[Slot::Reference(None), Slot::Reference(None)],
            &mut heap,
            &mut out,
        );
        assert!(result.is_ok());
        assert_eq!(String::from_utf8(out).unwrap(), "null");
    }

    // ---------------------------------------------------------------------------
    // native_println_boolean / native_print_boolean
    // ---------------------------------------------------------------------------

    #[test]
    fn native_println_boolean_false() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        native_println_boolean(&[Slot::Reference(None), Slot::Int(0)], &mut heap, &mut out)
            .unwrap();
        assert_eq!(String::from_utf8(out).unwrap().trim(), "false");
    }

    #[test]
    fn native_println_boolean_true() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        native_println_boolean(&[Slot::Reference(None), Slot::Int(1)], &mut heap, &mut out)
            .unwrap();
        assert_eq!(String::from_utf8(out).unwrap().trim(), "true");
    }

    #[test]
    fn native_print_boolean_false() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        native_print_boolean(&[Slot::Reference(None), Slot::Int(0)], &mut heap, &mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "false");
    }

    #[test]
    fn native_print_boolean_true() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        native_print_boolean(&[Slot::Reference(None), Slot::Int(5)], &mut heap, &mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "true");
    }

    // ---------------------------------------------------------------------------
    // native_print_char / native_print_long / native_print_float / native_print_double
    // ---------------------------------------------------------------------------

    #[test]
    fn native_print_char_ascii() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        native_print_char(
            &[Slot::Reference(None), Slot::Int('Z' as i32)],
            &mut heap,
            &mut out,
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "Z");
    }

    #[test]
    fn native_print_long_value() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        native_print_long(
            &[Slot::Reference(None), Slot::Long(123_456_789_000_i64)],
            &mut heap,
            &mut out,
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "123456789000");
    }

    #[test]
    fn native_print_float_value() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        native_print_float(
            &[Slot::Reference(None), Slot::Float(3.0_f32)],
            &mut heap,
            &mut out,
        )
        .unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains('3'), "expected '3' in '{s}'");
    }

    #[test]
    fn native_print_int_value() {
        let mut heap = duke_gc::Heap::new();
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        native_print_int(
            &[Slot::Reference(None), Slot::Int(42)],
            &mut heap,
            &mut out,
            &mut control,
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "42");
    }

    #[test]
    fn native_print_int_type_mismatch() {
        let mut heap = duke_gc::Heap::new();
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        let err = native_print_int(
            &[Slot::Reference(None), Slot::Long(42)],
            &mut heap,
            &mut out,
            &mut control,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::TypeMismatch { .. }));
    }

    #[test]
    fn native_println_int_value() {
        let mut heap = duke_gc::Heap::new();
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        native_println_int(
            &[Slot::Reference(None), Slot::Int(100)],
            &mut heap,
            &mut out,
            &mut control,
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "100\n");
    }

    #[test]
    fn native_println_int_type_mismatch() {
        let mut heap = duke_gc::Heap::new();
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        let err = native_println_int(
            &[Slot::Reference(None), Slot::Float(100.0)],
            &mut heap,
            &mut out,
            &mut control,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::TypeMismatch { .. }));
    }

    #[test]
    fn native_print_double_value() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        native_print_double(
            &[Slot::Reference(None), Slot::Double(2.5)],
            &mut heap,
            &mut out,
        )
        .unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("2.5"), "expected '2.5' in '{s}'");
    }

    // ---------------------------------------------------------------------------
    // native_println_object / native_print_object
    // ---------------------------------------------------------------------------

    #[test]
    fn native_println_object_null_prints_null() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        native_println_object(
            &[Slot::Reference(None), Slot::Reference(None)],
            &mut heap,
            &mut out,
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap().trim(), "null");
    }

    #[test]
    fn native_println_object_nonnull_uses_string_value() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate_string("hi".to_string());
        let mut out: Vec<u8> = Vec::new();
        native_println_object(
            &[Slot::Reference(None), Slot::Reference(Some(r))],
            &mut heap,
            &mut out,
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap().trim(), "hi");
    }

    #[test]
    fn native_print_object_null_prints_null() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        native_print_object(
            &[Slot::Reference(None), Slot::Reference(None)],
            &mut heap,
            &mut out,
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "null");
    }

    #[test]
    fn native_print_object_nonnull() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate_string("world".to_string());
        let mut out: Vec<u8> = Vec::new();
        native_print_object(
            &[Slot::Reference(None), Slot::Reference(Some(r))],
            &mut heap,
            &mut out,
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "world");
    }

    // ---------------------------------------------------------------------------
    // native_object_tostring
    // ---------------------------------------------------------------------------

    #[test]
    fn native_object_tostring_returns_string_ref() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate_string("test".to_string());
        let mut out: Vec<u8> = Vec::new();
        let result = native_object_tostring(&[Slot::Reference(Some(r))], &mut heap, &mut out)
            .unwrap()
            .unwrap();
        let new_ref = result.as_reference().unwrap();
        let s = heap.get(new_ref).unwrap().string_value.as_deref().unwrap();
        assert_eq!(s, "test");
    }

    // ---------------------------------------------------------------------------
    // native_string_equals null arm
    // ---------------------------------------------------------------------------

    #[test]
    fn native_string_equals_null_other_returns_false() {
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("hello".to_string());
        let mut out: Vec<u8> = Vec::new();
        let result = native_string_equals(
            &[Slot::Reference(Some(this)), Slot::Reference(None)],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        assert_eq!(result, Slot::Int(0));
    }

    // ---------------------------------------------------------------------------
    // native_system_exit
    // ---------------------------------------------------------------------------

    #[test]
    fn native_system_exit_returns_system_exit_error() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let err = native_system_exit(&[Slot::Int(42)], &mut heap, &mut out).unwrap_err();
        assert!(
            matches!(err, VmError::SystemExit { code: 42 }),
            "expected SystemExit(42), got {err:?}"
        );
    }

    #[test]
    fn native_system_exit_non_int_arg_uses_code_1() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let err = native_system_exit(&[], &mut heap, &mut out).unwrap_err();
        assert!(
            matches!(err, VmError::SystemExit { code: 1 }),
            "empty args → SystemExit(1), got {err:?}"
        );
    }

    // ---------------------------------------------------------------------------
    // native_string_substring boundary / error
    // ---------------------------------------------------------------------------

    #[test]
    fn native_string_substring_full_string() {
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("hello".to_string());
        let mut out: Vec<u8> = Vec::new();
        let r = native_string_substring(
            &[Slot::Reference(Some(this)), Slot::Int(0)],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        let sub_ref = r.as_reference().unwrap();
        assert_eq!(
            heap.get(sub_ref).unwrap().string_value.as_deref(),
            Some("hello")
        );
    }

    #[test]
    fn native_string_substring_tail() {
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("hello".to_string());
        let mut out: Vec<u8> = Vec::new();
        let r = native_string_substring(
            &[Slot::Reference(Some(this)), Slot::Int(2)],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        let sub_ref = r.as_reference().unwrap();
        assert_eq!(
            heap.get(sub_ref).unwrap().string_value.as_deref(),
            Some("llo")
        );
    }

    #[test]
    fn native_string_substring_out_of_bounds_returns_error() {
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("hi".to_string());
        let mut out: Vec<u8> = Vec::new();
        let err = native_string_substring(
            &[Slot::Reference(Some(this)), Slot::Int(10)],
            &mut heap,
            &mut out,
        )
        .unwrap_err();
        assert!(
            matches!(err, VmError::ArrayIndexOutOfBounds { .. }),
            "expected ArrayIndexOutOfBounds, got {err:?}"
        );
    }

    #[test]
    fn native_string_substring_range_basic() {
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("hello".to_string());
        let mut out: Vec<u8> = Vec::new();
        let r = native_string_substring_range(
            &[Slot::Reference(Some(this)), Slot::Int(1), Slot::Int(4)],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        let sub_ref = r.as_reference().unwrap();
        assert_eq!(
            heap.get(sub_ref).unwrap().string_value.as_deref(),
            Some("ell")
        );
    }

    #[test]
    fn native_string_substring_range_out_of_bounds() {
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("hi".to_string());
        let mut out: Vec<u8> = Vec::new();
        let err = native_string_substring_range(
            &[Slot::Reference(Some(this)), Slot::Int(0), Slot::Int(10)],
            &mut heap,
            &mut out,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---------------------------------------------------------------------------
    // format_arg
    // ---------------------------------------------------------------------------

    #[test]
    fn format_arg_long_d_spec() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Long(12345_i64);
        let result = format_arg('d', None, &Slot::Reference(Some(r)), &heap).unwrap();
        assert_eq!(result, "12345");
    }

    #[test]
    fn format_arg_double_f_spec() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Double(std::f64::consts::PI);
        let result = format_arg('f', Some(2), &Slot::Reference(Some(r)), &heap).unwrap();
        assert_eq!(result, "3.14");
    }

    #[test]
    fn format_arg_float_f_spec() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Float".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Float(1.5_f32);
        let result = format_arg('f', Some(1), &Slot::Reference(Some(r)), &heap).unwrap();
        assert_eq!(result, "1.5");
    }

    #[test]
    fn format_arg_int_x_spec() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Int(255);
        let result = format_arg('x', None, &Slot::Reference(Some(r)), &heap).unwrap();
        assert_eq!(result, "ff");
    }

    #[test]
    fn format_arg_int_uppercase_x_spec() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Int(255);
        let result = format_arg('X', None, &Slot::Reference(Some(r)), &heap).unwrap();
        assert_eq!(result, "FF");
    }

    #[test]
    fn format_arg_long_x_spec() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Long(255_i64);
        let result = format_arg('x', None, &Slot::Reference(Some(r)), &heap).unwrap();
        assert_eq!(result, "ff");
    }

    #[test]
    fn format_arg_null_returns_null_string() {
        let heap = duke_gc::Heap::new();
        let result = format_arg('s', None, &Slot::Reference(None), &heap).unwrap();
        assert_eq!(result, "null");
    }

    // ---------------------------------------------------------------------------
    // native_string_format
    // ---------------------------------------------------------------------------

    #[test]
    fn native_string_format_percent_d() {
        let mut heap = duke_gc::Heap::new();
        let fmt_ref = heap.allocate_string("%d".to_string());
        let arr_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 1);
        let int_obj = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(int_obj).unwrap().fields[0] = Slot::Int(7);
        heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(int_obj));
        let mut out: Vec<u8> = Vec::new();
        let result = native_string_format(
            &[
                Slot::Reference(Some(fmt_ref)),
                Slot::Reference(Some(arr_ref)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        let s_ref = result.as_reference().unwrap();
        assert_eq!(heap.get(s_ref).unwrap().string_value.as_deref(), Some("7"));
    }

    #[test]
    fn native_string_format_percent_n() {
        let mut heap = duke_gc::Heap::new();
        let fmt_ref = heap.allocate_string("a%nb".to_string());
        let arr_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 0);
        let mut out: Vec<u8> = Vec::new();
        let result = native_string_format(
            &[
                Slot::Reference(Some(fmt_ref)),
                Slot::Reference(Some(arr_ref)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        let s_ref = result.as_reference().unwrap();
        assert_eq!(
            heap.get(s_ref).unwrap().string_value.as_deref(),
            Some("a\nb")
        );
    }

    #[test]
    fn native_string_format_percent_percent() {
        let mut heap = duke_gc::Heap::new();
        let fmt_ref = heap.allocate_string("100%%".to_string());
        let arr_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 0);
        let mut out: Vec<u8> = Vec::new();
        let result = native_string_format(
            &[
                Slot::Reference(Some(fmt_ref)),
                Slot::Reference(Some(arr_ref)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        let s_ref = result.as_reference().unwrap();
        assert_eq!(
            heap.get(s_ref).unwrap().string_value.as_deref(),
            Some("100%")
        );
    }

    #[test]
    fn native_string_format_precision() {
        let mut heap = duke_gc::Heap::new();
        let fmt_ref = heap.allocate_string("%.2f".to_string());
        let arr_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 1);
        let dbl_obj = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(dbl_obj).unwrap().fields[0] = Slot::Double(std::f64::consts::PI);
        heap.get_mut(arr_ref).unwrap().fields[0] = Slot::Reference(Some(dbl_obj));
        let mut out: Vec<u8> = Vec::new();
        let result = native_string_format(
            &[
                Slot::Reference(Some(fmt_ref)),
                Slot::Reference(Some(arr_ref)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        let s_ref = result.as_reference().unwrap();
        assert_eq!(
            heap.get(s_ref).unwrap().string_value.as_deref(),
            Some("3.14")
        );
    }

    // ---------------------------------------------------------------------------
    // execute_string_concat_recipe
    // ---------------------------------------------------------------------------

    #[test]
    fn execute_string_concat_recipe_single_dynamic_int() {
        let mut heap = duke_gc::Heap::new();
        let slot = execute_string_concat_recipe("\u{1}", &[Slot::Int(42)], &['I'], &[], &mut heap)
            .unwrap();
        let r = slot.as_reference().unwrap();
        assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("42"));
    }

    #[test]
    fn execute_string_concat_recipe_constant_only() {
        let mut heap = duke_gc::Heap::new();
        let slot =
            execute_string_concat_recipe("\u{2}", &[], &[], &["hello".to_string()], &mut heap)
                .unwrap();
        let r = slot.as_reference().unwrap();
        assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("hello"));
    }

    #[test]
    fn execute_string_concat_recipe_literal_chars() {
        let mut heap = duke_gc::Heap::new();
        let slot = execute_string_concat_recipe("xyz", &[], &[], &[], &mut heap).unwrap();
        let r = slot.as_reference().unwrap();
        assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("xyz"));
    }

    #[test]
    fn execute_string_concat_recipe_mixed() {
        let mut heap = duke_gc::Heap::new();
        let slot = execute_string_concat_recipe("x\u{1}y", &[Slot::Int(5)], &['I'], &[], &mut heap)
            .unwrap();
        let r = slot.as_reference().unwrap();
        assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("x5y"));
    }

    #[test]
    fn execute_string_concat_recipe_multiple_dynamics() {
        let mut heap = duke_gc::Heap::new();
        let slot = execute_string_concat_recipe(
            "\u{1}+\u{1}",
            &[Slot::Int(3), Slot::Int(4)],
            &['I', 'I'],
            &[],
            &mut heap,
        )
        .unwrap();
        let r = slot.as_reference().unwrap();
        assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("3+4"));
    }

    // ---------------------------------------------------------------------------
    // Ishr / Iushr masking via execute()
    // ---------------------------------------------------------------------------

    #[test]
    fn execute_ishr_masks_shift_amount() {
        // Arithmetic right shift: -8 >> 1 = -4 (sign-extending)
        // Also verifies s & 0x1F: shift=1 means bit mask = 1
        let instrs = vec![
            (0, Instruction::Bipush(-8_i8)),
            (2, Instruction::Bipush(1_i8)),
            (4, Instruction::Ishr),
            (5, Instruction::Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 4, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(-4)));
    }

    #[test]
    fn execute_ishr_large_shift_masked_to_31() {
        // shift = 33 → 33 & 0x1F = 1; -8 >> 1 = -4
        let instrs = vec![
            (0, Instruction::Bipush(-8_i8)),
            (2, Instruction::Bipush(33_i8)),
            (4, Instruction::Ishr),
            (5, Instruction::Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 4, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(-4)));
    }

    #[test]
    fn execute_iushr_masks_shift_amount() {
        // Logical right shift: -1 (0xFFFFFFFF) >>> 28 = 15
        let instrs = vec![
            (0, Instruction::IconstM1),
            (1, Instruction::Bipush(28_i8)),
            (3, Instruction::Iushr),
            (4, Instruction::Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 4, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(15)));
    }

    #[test]
    fn execute_iushr_large_shift_masked() {
        // shift=60 → 60 & 0x1F = 28; -1 >>> 28 = 15
        let instrs = vec![
            (0, Instruction::IconstM1),
            (1, Instruction::Bipush(60_i8)),
            (3, Instruction::Iushr),
            (4, Instruction::Ireturn),
        ];
        let result = execute(&instrs, &[], vec![], 4, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(15)));
    }

    // ---------------------------------------------------------------------------
    // Lshr / Lushr masking via execute()
    // ---------------------------------------------------------------------------

    #[test]
    fn execute_lshr_arithmetic_shift() {
        // long -8 >> 1 = -4 (arithmetic; sign bit preserved)
        let instrs = vec![
            (0, Instruction::Lload0),
            (1, Instruction::Bipush(1_i8)),
            (3, Instruction::Lshr),
            (4, Instruction::Lreturn),
        ];
        let result = execute(&instrs, &[], vec![Slot::Long(-8_i64)], 4, 2).unwrap();
        assert_eq!(result, Some(Slot::Long(-4)));
    }

    #[test]
    fn execute_lushr_logical_shift() {
        // long -1 (0xFFFFFFFFFFFFFFFF) >>> 60 = 15
        use duke_bytecode::Instruction;
        let instrs = vec![
            (0, Instruction::Lload0),
            (1, Instruction::Bipush(60_i8)),
            (3, Instruction::Lushr),
            (4, Instruction::Lreturn),
        ];
        let result = execute(&instrs, &[], vec![Slot::Long(-1_i64)], 4, 2).unwrap();
        assert_eq!(result, Some(Slot::Long(15)));
    }

    #[test]
    fn execute_lshr_large_shift_masked_to_63() {
        // shift = 65 → 65 & 0x3F = 1; -8 >> 1 = -4
        use duke_bytecode::Instruction;
        let instrs = vec![
            (0, Instruction::Lload0),
            (1, Instruction::Bipush(65_i8)),
            (3, Instruction::Lshr),
            (4, Instruction::Lreturn),
        ];
        let result = execute(&instrs, &[], vec![Slot::Long(-8_i64)], 4, 2).unwrap();
        assert_eq!(result, Some(Slot::Long(-4)));
    }

    // ---------------------------------------------------------------------------
    // LDC CpEntry::Utf8 arm — via execute() with a hand-crafted CP
    // ---------------------------------------------------------------------------

    #[test]
    fn execute_ldc_string_from_utf8_cp() {
        use duke_classfile::types::{CpEntry, CpIndex};
        // CP: [None, Some(String{string_index:2}), Some(Utf8("hi"))]
        let cp: Vec<Option<CpEntry>> = vec![
            None,
            Some(CpEntry::String {
                string_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("hi".to_string())),
        ];
        // Ldc takes u8 cp index; execute() doesn't implement Areturn so just Pop+Iconst0+Ireturn
        let instrs = vec![
            (0, Instruction::Ldc(1u8)),
            (2, Instruction::Pop),
            (3, Instruction::Iconst0),
            (4, Instruction::Ireturn),
        ];
        // Must not error — exercises the CpEntry::Utf8 branch
        let result = execute(&instrs, &cp, vec![], 2, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(0)));
    }

    #[test]
    fn execute_ldcw_string_from_utf8_cp() {
        use duke_classfile::types::{CpEntry, CpIndex};
        let cp: Vec<Option<CpEntry>> = vec![
            None,
            Some(CpEntry::String {
                string_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("world".to_string())),
        ];
        let instrs = vec![
            (0, Instruction::LdcW(CpIndex(1))),
            (3, Instruction::Pop),
            (4, Instruction::Iconst1),
            (5, Instruction::Ireturn),
        ];
        let result = execute(&instrs, &cp, vec![], 2, 1).unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    // ---------------------------------------------------------------------------
    // native_hashmap_remove
    // ---------------------------------------------------------------------------

    #[test]
    fn native_hashmap_remove_existing_key_returns_value_and_decrements_size() {
        let mut heap = duke_gc::Heap::new();
        // Build map: fields = [Int(1), key_ref, val_ref]
        let map = heap.allocate("java/util/HashMap".to_string(), 1);
        let key = heap.allocate_string("k".to_string());
        let val = heap.allocate_string("v".to_string());
        {
            let obj = heap.get_mut(map).unwrap();
            obj.fields[0] = Slot::Int(1); // size
            obj.fields.push(Slot::Reference(Some(key)));
            obj.fields.push(Slot::Reference(Some(val)));
        }
        let mut out: Vec<u8> = Vec::new();
        let result = native_hashmap_remove(
            &[Slot::Reference(Some(map)), Slot::Reference(Some(key))],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        assert_eq!(result, Slot::Reference(Some(val)));
        // Size should be 0 now
        assert_eq!(heap.get(map).unwrap().fields[0], Slot::Int(0));
    }

    #[test]
    fn native_hashmap_remove_absent_key_returns_null() {
        let mut heap = duke_gc::Heap::new();
        let map = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map).unwrap().fields[0] = Slot::Int(0);
        let missing_key = heap.allocate_string("missing".to_string());
        let mut out: Vec<u8> = Vec::new();
        let result = native_hashmap_remove(
            &[
                Slot::Reference(Some(map)),
                Slot::Reference(Some(missing_key)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        assert_eq!(result, Slot::Reference(None));
    }

    // ---------------------------------------------------------------------------
    // native_hashmap_get_or_default
    // ---------------------------------------------------------------------------

    #[test]
    fn native_hashmap_get_or_default_key_present() {
        let mut heap = duke_gc::Heap::new();
        let map = heap.allocate("java/util/HashMap".to_string(), 1);
        let key = heap.allocate_string("k".to_string());
        let val = heap.allocate_string("v".to_string());
        let def = heap.allocate_string("default".to_string());
        {
            let obj = heap.get_mut(map).unwrap();
            obj.fields[0] = Slot::Int(1);
            obj.fields.push(Slot::Reference(Some(key)));
            obj.fields.push(Slot::Reference(Some(val)));
        }
        let mut out: Vec<u8> = Vec::new();
        let result = native_hashmap_get_or_default(
            &[
                Slot::Reference(Some(map)),
                Slot::Reference(Some(key)),
                Slot::Reference(Some(def)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        assert_eq!(result, Slot::Reference(Some(val)));
    }

    #[test]
    fn native_hashmap_get_or_default_key_absent_returns_default() {
        let mut heap = duke_gc::Heap::new();
        let map = heap.allocate("java/util/HashMap".to_string(), 1);
        heap.get_mut(map).unwrap().fields[0] = Slot::Int(0);
        let missing = heap.allocate_string("nope".to_string());
        let def = heap.allocate_string("fallback".to_string());
        let mut out: Vec<u8> = Vec::new();
        let result = native_hashmap_get_or_default(
            &[
                Slot::Reference(Some(map)),
                Slot::Reference(Some(missing)),
                Slot::Reference(Some(def)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        assert_eq!(result, Slot::Reference(Some(def)));
    }

    // ---------------------------------------------------------------------------
    // null-arm tests: String.indexOf, String.contains
    // ---------------------------------------------------------------------------

    #[test]
    fn native_string_indexof_null_target_raises_npe() {
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("hello".to_string());
        let mut out: Vec<u8> = Vec::new();
        let err = native_string_indexof(
            &[Slot::Reference(Some(this)), Slot::Reference(None)],
            &mut heap,
            &mut out,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::NullPointerException));
    }

    #[test]
    fn native_string_contains_null_target_raises_npe() {
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("hello".to_string());
        let mut out: Vec<u8> = Vec::new();
        let err = native_string_contains(
            &[Slot::Reference(Some(this)), Slot::Reference(None)],
            &mut heap,
            &mut out,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::NullPointerException));
    }

    // ---------------------------------------------------------------------------
    // null-arm: Integer.parseInt, Long.parseLong, Double.parseDouble,
    //           Float.parseFloat, Boolean.parseBoolean
    // ---------------------------------------------------------------------------

    #[test]
    fn native_integer_parseint_null_raises_npe() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let err =
            native_integer_parseint(&[Slot::Reference(None)], &mut heap, &mut out).unwrap_err();
        assert!(matches!(err, VmError::NullPointerException));
    }

    #[test]
    fn native_integer_parseint_valid_string() {
        let mut heap = duke_gc::Heap::new();
        let s = heap.allocate_string("42".to_string());
        let mut out: Vec<u8> = Vec::new();
        let r = native_integer_parseint(&[Slot::Reference(Some(s))], &mut heap, &mut out)
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(42));
    }

    #[test]
    fn native_long_parselong_null_raises_npe() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let err = native_long_parselong(&[Slot::Reference(None)], &mut heap, &mut out).unwrap_err();
        assert!(matches!(err, VmError::NullPointerException));
    }

    #[test]
    fn native_long_parselong_valid_string() {
        let mut heap = duke_gc::Heap::new();
        let s = heap.allocate_string("99999999999".to_string());
        let mut out: Vec<u8> = Vec::new();
        let r = native_long_parselong(&[Slot::Reference(Some(s))], &mut heap, &mut out)
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Long(99_999_999_999_i64));
    }

    #[test]
    fn native_double_parsedouble_null_raises_npe() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let err =
            native_double_parsedouble(&[Slot::Reference(None)], &mut heap, &mut out).unwrap_err();
        assert!(matches!(err, VmError::NullPointerException));
    }

    #[test]
    fn native_double_parsedouble_valid_string() {
        let mut heap = duke_gc::Heap::new();
        let s = heap.allocate_string("2.5".to_string());
        let mut out: Vec<u8> = Vec::new();
        let r = native_double_parsedouble(&[Slot::Reference(Some(s))], &mut heap, &mut out)
            .unwrap()
            .unwrap();
        assert!(matches!(r, Slot::Double(v) if (v - 2.5).abs() < 1e-9));
    }

    #[test]
    fn native_double_doublevalue_nonnull() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Double".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Double(2.5);
        let mut out: Vec<u8> = Vec::new();
        let result = native_double_doublevalue(&[Slot::Reference(Some(r))], &mut heap, &mut out)
            .unwrap()
            .unwrap();
        assert!(matches!(result, Slot::Double(v) if (v - 2.5).abs() < 1e-9));
    }

    #[test]
    fn native_float_parsefloat_null_raises_npe() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let err =
            native_float_parsefloat(&[Slot::Reference(None)], &mut heap, &mut out).unwrap_err();
        assert!(matches!(err, VmError::NullPointerException));
    }

    #[test]
    fn native_float_parsefloat_valid_string() {
        let mut heap = duke_gc::Heap::new();
        let s = heap.allocate_string("1.5".to_string());
        let mut out: Vec<u8> = Vec::new();
        let r = native_float_parsefloat(&[Slot::Reference(Some(s))], &mut heap, &mut out)
            .unwrap()
            .unwrap();
        assert!(matches!(r, Slot::Float(v) if (v - 1.5_f32).abs() < 1e-6));
    }

    #[test]
    fn native_boolean_parseboolean_null_returns_false() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let r = native_boolean_parseboolean(&[Slot::Reference(None)], &mut heap, &mut out)
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn native_boolean_parseboolean_true_string() {
        let mut heap = duke_gc::Heap::new();
        let s = heap.allocate_string("true".to_string());
        let mut out: Vec<u8> = Vec::new();
        let r = native_boolean_parseboolean(&[Slot::Reference(Some(s))], &mut heap, &mut out)
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn native_boolean_parseboolean_false_string() {
        let mut heap = duke_gc::Heap::new();
        let s = heap.allocate_string("false".to_string());
        let mut out: Vec<u8> = Vec::new();
        let r = native_boolean_parseboolean(&[Slot::Reference(Some(s))], &mut heap, &mut out)
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    // ---------------------------------------------------------------------------
    // null-arm: String.valueOf(Object) null/nonnull
    // ---------------------------------------------------------------------------

    #[test]
    fn native_string_value_of_object_null_returns_null_string() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let r = native_string_value_of_object(&[Slot::Reference(None)], &mut heap, &mut out)
            .unwrap()
            .unwrap();
        let ref_r = r.as_reference().unwrap();
        assert_eq!(
            heap.get(ref_r).unwrap().string_value.as_deref(),
            Some("null")
        );
    }

    #[test]
    fn native_string_value_of_object_nonnull() {
        let mut heap = duke_gc::Heap::new();
        let obj = heap.allocate_string("hello".to_string());
        let mut out: Vec<u8> = Vec::new();
        let r = native_string_value_of_object(&[Slot::Reference(Some(obj))], &mut heap, &mut out)
            .unwrap()
            .unwrap();
        let ref_r = r.as_reference().unwrap();
        assert_eq!(
            heap.get(ref_r).unwrap().string_value.as_deref(),
            Some("hello")
        );
    }

    // ---------------------------------------------------------------------------
    // null-arm: String.concat, String.replace(CharSequence), String.split
    // ---------------------------------------------------------------------------

    #[test]
    fn native_string_concat_null_other_raises_npe() {
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("hello".to_string());
        let mut out: Vec<u8> = Vec::new();
        let err = native_string_concat(
            &[Slot::Reference(Some(this)), Slot::Reference(None)],
            &mut heap,
            &mut out,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::NullPointerException));
    }

    #[test]
    fn native_string_replace_charsequence_null_target_raises_npe() {
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("hello".to_string());
        let rep = heap.allocate_string("x".to_string());
        let mut out: Vec<u8> = Vec::new();
        let err = native_string_replace_charsequence(
            &[
                Slot::Reference(Some(this)),
                Slot::Reference(None),
                Slot::Reference(Some(rep)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::NullPointerException));
    }

    #[test]
    fn native_string_split_null_delimiter_raises_npe() {
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("a,b".to_string());
        let mut out: Vec<u8> = Vec::new();
        let err = native_string_split(
            &[Slot::Reference(Some(this)), Slot::Reference(None)],
            &mut heap,
            &mut out,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::NullPointerException));
    }

    // ---------------------------------------------------------------------------
    // native_math_min_double
    // ---------------------------------------------------------------------------

    #[test]
    fn native_math_min_double_returns_smaller() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let r =
            native_math_min_double(&[Slot::Double(3.0), Slot::Double(1.5)], &mut heap, &mut out)
                .unwrap()
                .unwrap();
        assert!(matches!(r, Slot::Double(v) if (v - 1.5).abs() < 1e-9));
    }

    #[test]
    fn native_math_min_double_returns_first_when_equal() {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let r =
            native_math_min_double(&[Slot::Double(2.0), Slot::Double(2.0)], &mut heap, &mut out)
                .unwrap()
                .unwrap();
        assert!(matches!(r, Slot::Double(v) if (v - 2.0).abs() < 1e-9));
    }

    // ===========================================================================
    // execute() — Ixor
    // ===========================================================================

    #[test]
    fn execute_ixor_xors_bits() {
        let r = execute(
            &[
                (0, Instruction::Bipush(5_i8)),
                (2, Instruction::Bipush(3_i8)),
                (4, Instruction::Ixor),
                (5, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(6)); // 5 ^ 3 = 6
    }

    #[test]
    fn execute_ixor_self_gives_zero() {
        let r = execute(
            &[
                (0, Instruction::Bipush(9_i8)),
                (2, Instruction::Bipush(9_i8)),
                (4, Instruction::Ixor),
                (5, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    // ===========================================================================
    // execute() — Long bitwise: Land / Lor / Lxor / Lneg / Lshl masking
    // ===========================================================================

    #[test]
    fn execute_land_selects_common_bits() {
        // 0b1010 & 0b1100 = 0b1000 = 8
        let r = execute(
            &[
                (0, Instruction::Lload0),
                (1, Instruction::Lload1),
                (2, Instruction::Land),
                (3, Instruction::Lreturn),
            ],
            &[],
            vec![Slot::Long(0b1010), Slot::Long(0b1100)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(8));
    }

    #[test]
    fn execute_lor_combines_bits() {
        // 0b1010 | 0b0101 = 0b1111 = 15
        let r = execute(
            &[
                (0, Instruction::Lload0),
                (1, Instruction::Lload1),
                (2, Instruction::Lor),
                (3, Instruction::Lreturn),
            ],
            &[],
            vec![Slot::Long(0b1010), Slot::Long(0b0101)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(15));
    }

    #[test]
    fn execute_lxor_flips_differing_bits() {
        // 0b1010 ^ 0b1100 = 0b0110 = 6
        let r = execute(
            &[
                (0, Instruction::Lload0),
                (1, Instruction::Lload1),
                (2, Instruction::Lxor),
                (3, Instruction::Lreturn),
            ],
            &[],
            vec![Slot::Long(0b1010), Slot::Long(0b1100)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(6));
    }

    #[test]
    fn execute_lneg_negates_value() {
        let r = execute(
            &[
                (0, Instruction::Lload0),
                (1, Instruction::Lneg),
                (2, Instruction::Lreturn),
            ],
            &[],
            vec![Slot::Long(1)],
            4,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(-1));
    }

    #[test]
    fn execute_lshl_masks_shift_amount_distinguishes_and_from_xor() {
        // shift by 65: 65 & 63 = 1 → 1L << 1 = 2
        // with ^ mutant: 65 ^ 63 = 64 → shift by 0 → 1
        let r = execute(
            &[
                (0, Instruction::Lload0),
                (1, Instruction::Iload1),
                (2, Instruction::Lshl),
                (3, Instruction::Lreturn),
            ],
            &[],
            vec![Slot::Long(1), Slot::Int(65)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(2));
    }

    // ===========================================================================
    // execute() — Float arithmetic
    // ===========================================================================

    #[test]
    fn execute_fadd_sums_floats() {
        let r = execute(
            &[
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fadd),
                (3, Instruction::Freturn),
            ],
            &[],
            vec![Slot::Float(2.0), Slot::Float(3.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Float(v) if (v - 5.0).abs() < 1e-6));
    }

    #[test]
    fn execute_fsub_subtracts_floats() {
        let r = execute(
            &[
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fsub),
                (3, Instruction::Freturn),
            ],
            &[],
            vec![Slot::Float(5.0), Slot::Float(3.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Float(v) if (v - 2.0).abs() < 1e-6));
    }

    #[test]
    fn execute_fmul_multiplies_floats() {
        let r = execute(
            &[
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fmul),
                (3, Instruction::Freturn),
            ],
            &[],
            vec![Slot::Float(3.0), Slot::Float(4.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Float(v) if (v - 12.0).abs() < 1e-6));
    }

    #[test]
    fn execute_fdiv_divides_floats() {
        let r = execute(
            &[
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fdiv),
                (3, Instruction::Freturn),
            ],
            &[],
            vec![Slot::Float(6.0), Slot::Float(2.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Float(v) if (v - 3.0).abs() < 1e-6));
    }

    #[test]
    fn execute_frem_float_remainder() {
        let r = execute(
            &[
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Frem),
                (3, Instruction::Freturn),
            ],
            &[],
            vec![Slot::Float(7.0), Slot::Float(3.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Float(v) if (v - 1.0).abs() < 1e-6));
    }

    #[test]
    fn execute_fneg_negates_float() {
        let r = execute(
            &[
                (0, Instruction::Fload0),
                (1, Instruction::Fneg),
                (2, Instruction::Freturn),
            ],
            &[],
            vec![Slot::Float(5.0)],
            4,
            1,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Float(v) if (v + 5.0).abs() < 1e-6));
    }

    #[test]
    fn execute_fcmpg_greater_gives_1() {
        let r = execute(
            &[
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fcmpg),
                (3, Instruction::Ireturn),
            ],
            &[],
            vec![Slot::Float(3.0), Slot::Float(1.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_fcmpg_less_gives_minus1() {
        let r = execute(
            &[
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fcmpg),
                (3, Instruction::Ireturn),
            ],
            &[],
            vec![Slot::Float(1.0), Slot::Float(3.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(-1));
    }

    #[test]
    fn execute_fcmpg_equal_gives_0() {
        let r = execute(
            &[
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fcmpg),
                (3, Instruction::Ireturn),
            ],
            &[],
            vec![Slot::Float(2.0), Slot::Float(2.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_fcmpg_nan_gives_1() {
        let r = execute(
            &[
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fcmpg),
                (3, Instruction::Ireturn),
            ],
            &[],
            vec![Slot::Float(f32::NAN), Slot::Float(1.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_fcmpl_nan_gives_minus1() {
        let r = execute(
            &[
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fcmpl),
                (3, Instruction::Ireturn),
            ],
            &[],
            vec![Slot::Float(f32::NAN), Slot::Float(1.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(-1));
    }

    // ===========================================================================
    // execute() — Double arithmetic
    // ===========================================================================

    #[test]
    fn execute_dadd_sums_doubles() {
        let r = execute(
            &[
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dadd),
                (3, Instruction::Dreturn),
            ],
            &[],
            vec![Slot::Double(2.0), Slot::Double(3.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Double(v) if (v - 5.0).abs() < 1e-9));
    }

    #[test]
    fn execute_dsub_subtracts_doubles() {
        let r = execute(
            &[
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dsub),
                (3, Instruction::Dreturn),
            ],
            &[],
            vec![Slot::Double(5.0), Slot::Double(3.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Double(v) if (v - 2.0).abs() < 1e-9));
    }

    #[test]
    fn execute_dmul_multiplies_doubles() {
        let r = execute(
            &[
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dmul),
                (3, Instruction::Dreturn),
            ],
            &[],
            vec![Slot::Double(3.0), Slot::Double(4.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Double(v) if (v - 12.0).abs() < 1e-9));
    }

    #[test]
    fn execute_ddiv_divides_doubles() {
        let r = execute(
            &[
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Ddiv),
                (3, Instruction::Dreturn),
            ],
            &[],
            vec![Slot::Double(6.0), Slot::Double(2.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Double(v) if (v - 3.0).abs() < 1e-9));
    }

    #[test]
    fn execute_drem_double_remainder() {
        let r = execute(
            &[
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Drem),
                (3, Instruction::Dreturn),
            ],
            &[],
            vec![Slot::Double(7.0), Slot::Double(3.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Double(v) if (v - 1.0).abs() < 1e-9));
    }

    #[test]
    fn execute_dneg_negates_double() {
        let r = execute(
            &[
                (0, Instruction::Dload0),
                (1, Instruction::Dneg),
                (2, Instruction::Dreturn),
            ],
            &[],
            vec![Slot::Double(5.0)],
            4,
            1,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Double(v) if (v + 5.0).abs() < 1e-9));
    }

    #[test]
    fn execute_dcmpg_greater_gives_1() {
        let r = execute(
            &[
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dcmpg),
                (3, Instruction::Ireturn),
            ],
            &[],
            vec![Slot::Double(3.0), Slot::Double(1.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_dcmpg_less_gives_minus1() {
        let r = execute(
            &[
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dcmpg),
                (3, Instruction::Ireturn),
            ],
            &[],
            vec![Slot::Double(1.0), Slot::Double(3.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(-1));
    }

    #[test]
    fn execute_dcmpg_equal_gives_0() {
        let r = execute(
            &[
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dcmpg),
                (3, Instruction::Ireturn),
            ],
            &[],
            vec![Slot::Double(2.0), Slot::Double(2.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_dcmpg_nan_gives_1() {
        let r = execute(
            &[
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dcmpg),
                (3, Instruction::Ireturn),
            ],
            &[],
            vec![Slot::Double(f64::NAN), Slot::Double(1.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_dcmpl_nan_gives_minus1() {
        let r = execute(
            &[
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dcmpl),
                (3, Instruction::Ireturn),
            ],
            &[],
            vec![Slot::Double(f64::NAN), Slot::Double(1.0)],
            4,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(-1));
    }

    // ===========================================================================
    // execute() — Conditional branches
    // Pattern: push condition, Ifxx(offset=4) at PC=1 → target=5
    // ===========================================================================

    #[test]
    fn execute_ifne_taken_when_nonzero() {
        let r = execute(
            &[
                (0, Instruction::Bipush(5_i8)),
                (2, Instruction::Ifne(4)),
                (4, Instruction::Iconst0),
                (5, Instruction::Ireturn),
                (6, Instruction::Iconst1),
                (7, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_ifne_not_taken_when_zero() {
        let r = execute(
            &[
                (0, Instruction::Iconst0),
                (1, Instruction::Ifne(4)),
                (3, Instruction::Iconst0),
                (4, Instruction::Ireturn),
                (5, Instruction::Iconst1),
                (6, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_iflt_taken_when_negative() {
        let r = execute(
            &[
                (0, Instruction::IconstM1),
                (1, Instruction::Iflt(4)),
                (3, Instruction::Iconst0),
                (4, Instruction::Ireturn),
                (5, Instruction::Iconst1),
                (6, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_iflt_not_taken_when_zero() {
        let r = execute(
            &[
                (0, Instruction::Iconst0),
                (1, Instruction::Iflt(4)),
                (3, Instruction::Iconst0),
                (4, Instruction::Ireturn),
                (5, Instruction::Iconst1),
                (6, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_ifgt_taken_when_positive() {
        let r = execute(
            &[
                (0, Instruction::Iconst1),
                (1, Instruction::Ifgt(4)),
                (3, Instruction::Iconst0),
                (4, Instruction::Ireturn),
                (5, Instruction::Iconst1),
                (6, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_ifgt_not_taken_when_zero() {
        let r = execute(
            &[
                (0, Instruction::Iconst0),
                (1, Instruction::Ifgt(4)),
                (3, Instruction::Iconst0),
                (4, Instruction::Ireturn),
                (5, Instruction::Iconst1),
                (6, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_ifle_taken_when_negative() {
        let r = execute(
            &[
                (0, Instruction::IconstM1),
                (1, Instruction::Ifle(4)),
                (3, Instruction::Iconst0),
                (4, Instruction::Ireturn),
                (5, Instruction::Iconst1),
                (6, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_ifle_taken_when_zero() {
        let r = execute(
            &[
                (0, Instruction::Iconst0),
                (1, Instruction::Ifle(4)),
                (3, Instruction::Iconst0),
                (4, Instruction::Ireturn),
                (5, Instruction::Iconst1),
                (6, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_ifle_not_taken_when_positive() {
        let r = execute(
            &[
                (0, Instruction::Iconst1),
                (1, Instruction::Ifle(4)),
                (3, Instruction::Iconst0),
                (4, Instruction::Ireturn),
                (5, Instruction::Iconst1),
                (6, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_ifnull_taken_when_null() {
        let r = execute(
            &[
                (0, Instruction::AconstNull),
                (1, Instruction::Ifnull(4)),
                (3, Instruction::Iconst0),
                (4, Instruction::Ireturn),
                (5, Instruction::Iconst1),
                (6, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_ifnull_not_taken_when_nonnull() {
        let r = execute(
            &[
                (0, Instruction::Aload0),
                (1, Instruction::Ifnull(4)),
                (3, Instruction::Iconst1),
                (4, Instruction::Ireturn),
                (5, Instruction::Iconst0),
                (6, Instruction::Ireturn),
            ],
            &[],
            vec![Slot::Reference(Some(0))],
            4,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_ifnonnull_taken_when_nonnull() {
        let r = execute(
            &[
                (0, Instruction::Aload0),
                (1, Instruction::Ifnonnull(4)),
                (3, Instruction::Iconst0),
                (4, Instruction::Ireturn),
                (5, Instruction::Iconst1),
                (6, Instruction::Ireturn),
            ],
            &[],
            vec![Slot::Reference(Some(42))],
            4,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_ifnonnull_not_taken_when_null() {
        let r = execute(
            &[
                (0, Instruction::AconstNull),
                (1, Instruction::Ifnonnull(4)),
                (3, Instruction::Iconst0),
                (4, Instruction::Ireturn),
                (5, Instruction::Iconst1),
                (6, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_ificmplt_taken_when_a_less_than_b() {
        let r = execute(
            &[
                (0, Instruction::Iconst2),
                (1, Instruction::Iconst3),
                (2, Instruction::IfIcmplt(4)),
                (4, Instruction::Iconst0),
                (5, Instruction::Ireturn),
                (6, Instruction::Iconst1),
                (7, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1)); // 2 < 3 → taken
    }

    #[test]
    fn execute_ificmplt_not_taken_when_equal() {
        let r = execute(
            &[
                (0, Instruction::Iconst3),
                (1, Instruction::Iconst3),
                (2, Instruction::IfIcmplt(4)),
                (4, Instruction::Iconst0),
                (5, Instruction::Ireturn),
                (6, Instruction::Iconst1),
                (7, Instruction::Ireturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0)); // 3 < 3 is false → not taken
    }

    // ===========================================================================
    // execute() — Newarray Long/Float/Double initializes correct slot types
    // ===========================================================================

    #[test]
    fn execute_newarray_long_initializes_long_zero() {
        use duke_bytecode::instruction::ArrayType;
        let r = execute(
            &[
                (0, Instruction::Iconst1),
                (1, Instruction::Newarray(ArrayType::Long)),
                (3, Instruction::Dup),
                (4, Instruction::Iconst0),
                (5, Instruction::Laload),
                (6, Instruction::Lreturn),
            ],
            &[],
            vec![],
            8,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(0));
    }

    #[test]
    fn execute_newarray_float_initializes_float_zero() {
        use duke_bytecode::instruction::ArrayType;
        let r = execute(
            &[
                (0, Instruction::Iconst1),
                (1, Instruction::Newarray(ArrayType::Float)),
                (3, Instruction::Dup),
                (4, Instruction::Iconst0),
                (5, Instruction::Faload),
                (6, Instruction::Freturn),
            ],
            &[],
            vec![],
            8,
            0,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Float(v) if v == 0.0));
    }

    #[test]
    fn execute_newarray_double_initializes_double_zero() {
        use duke_bytecode::instruction::ArrayType;
        let r = execute(
            &[
                (0, Instruction::Iconst1),
                (1, Instruction::Newarray(ArrayType::Double)),
                (3, Instruction::Dup),
                (4, Instruction::Iconst0),
                (5, Instruction::Daload),
                (6, Instruction::Dreturn),
            ],
            &[],
            vec![],
            8,
            0,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Double(v) if v == 0.0));
    }

    // ===========================================================================
    // execute() — Long/Float/Double array store/load roundtrip + bounds
    // ===========================================================================

    #[test]
    fn execute_lastore_and_laload_roundtrip() {
        use duke_bytecode::instruction::ArrayType;
        // Create long[2], store Lconst1 at index 0, load and return it.
        let r = execute(
            &[
                (0, Instruction::Iconst2),
                (1, Instruction::Newarray(ArrayType::Long)),
                (3, Instruction::Dup),
                (4, Instruction::Iconst0),
                (5, Instruction::Lconst1),
                (6, Instruction::Lastore),
                (7, Instruction::Iconst0),
                (8, Instruction::Laload),
                (9, Instruction::Lreturn),
            ],
            &[],
            vec![],
            8,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(1));
    }

    #[test]
    fn execute_laload_out_of_bounds_raises_error() {
        use duke_bytecode::instruction::ArrayType;
        let err = execute(
            &[
                (0, Instruction::Iconst1),
                (1, Instruction::Newarray(ArrayType::Long)),
                (3, Instruction::Iconst2),
                (4, Instruction::Laload),
                (5, Instruction::Lreturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    #[test]
    fn execute_fastore_and_faload_roundtrip() {
        use duke_bytecode::instruction::ArrayType;
        let r = execute(
            &[
                (0, Instruction::Iconst1),
                (1, Instruction::Newarray(ArrayType::Float)),
                (3, Instruction::Dup),
                (4, Instruction::Iconst0),
                (5, Instruction::Fconst2),
                (6, Instruction::Fastore),
                (7, Instruction::Iconst0),
                (8, Instruction::Faload),
                (9, Instruction::Freturn),
            ],
            &[],
            vec![],
            8,
            0,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Float(v) if (v - 2.0).abs() < 1e-6));
    }

    #[test]
    fn execute_dastore_and_daload_roundtrip() {
        use duke_bytecode::instruction::ArrayType;
        let r = execute(
            &[
                (0, Instruction::Iconst1),
                (1, Instruction::Newarray(ArrayType::Double)),
                (3, Instruction::Dup),
                (4, Instruction::Iconst0),
                (5, Instruction::Dconst1),
                (6, Instruction::Dastore),
                (7, Instruction::Iconst0),
                (8, Instruction::Daload),
                (9, Instruction::Dreturn),
            ],
            &[],
            vec![],
            8,
            0,
        )
        .unwrap()
        .unwrap();
        assert!(matches!(r, Slot::Double(v) if (v - 1.0).abs() < 1e-9));
    }

    #[test]
    fn execute_faload_out_of_bounds_raises_error() {
        use duke_bytecode::instruction::ArrayType;
        let err = execute(
            &[
                (0, Instruction::Iconst1),
                (1, Instruction::Newarray(ArrayType::Float)),
                (3, Instruction::Sipush(5_i16)),
                (6, Instruction::Faload),
                (7, Instruction::Freturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    #[test]
    fn execute_daload_out_of_bounds_raises_error() {
        use duke_bytecode::instruction::ArrayType;
        let err = execute(
            &[
                (0, Instruction::Iconst1),
                (1, Instruction::Newarray(ArrayType::Double)),
                (3, Instruction::Sipush(10_i16)),
                (6, Instruction::Daload),
                (7, Instruction::Dreturn),
            ],
            &[],
            vec![],
            4,
            0,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ===========================================================================
    // FramePool: acquire / release / cap-at-256
    // ===========================================================================

    #[test]
    fn frame_pool_acquire_from_empty_gives_empty_vecs() {
        let mut pool = FramePool::new();
        let (locals, stack): (Vec<Slot>, Vec<Slot>) = pool.acquire();
        assert!(locals.is_empty());
        assert!(stack.is_empty());
    }

    #[test]
    fn frame_pool_release_and_reacquire_returns_pooled_bufs() {
        let mut pool = FramePool::new();
        let locals = vec![Slot::Int(1), Slot::Int(2)];
        let stack = vec![];
        pool.release(locals, stack);
        let (locals2, stack2): (Vec<Slot>, Vec<Slot>) = pool.acquire();
        assert_eq!(locals2.len(), 2);
        assert!(stack2.is_empty());
        // Pool should be empty again
        let (locals3, _) = pool.acquire();
        let locals3_copy: Vec<Slot> = locals3;
        assert!(locals3_copy.is_empty());
    }

    #[test]
    fn frame_pool_release_caps_at_256() {
        let mut pool = FramePool::new();
        for i in 0..300_i32 {
            pool.release(vec![Slot::Int(i)], vec![]);
        }
        assert_eq!(pool.free.len(), 256);
    }

    #[test]
    fn frame_pool_release_below_256_keeps_all() {
        let mut pool = FramePool::new();
        for i in 0..10_i32 {
            pool.release(vec![Slot::Int(i)], vec![]);
        }
        assert_eq!(pool.free.len(), 10);
    }

    // ===========================================================================
    // Helper functions: parse_arg_count / parse_arg_types
    // ===========================================================================

    #[test]
    fn parse_arg_count_primitives() {
        assert_eq!(parse_arg_count("(IZB)V"), 3);
        assert_eq!(parse_arg_count("(JFDS)V"), 4);
        assert_eq!(parse_arg_count("()V"), 0);
        assert_eq!(parse_arg_count("(I)I"), 1);
    }

    #[test]
    fn parse_arg_count_object_type_counts_once() {
        assert_eq!(parse_arg_count("(Ljava/lang/String;)V"), 1);
        assert_eq!(parse_arg_count("(Ljava/lang/String;I)V"), 2);
    }

    #[test]
    fn parse_arg_count_array_types() {
        assert_eq!(parse_arg_count("([I)V"), 1);
        assert_eq!(parse_arg_count("([Ljava/lang/String;)V"), 1);
        assert_eq!(parse_arg_count("([[I)V"), 1); // 2-D array = 1 slot
        assert_eq!(parse_arg_count("([I[Z)V"), 2);
    }

    #[test]
    fn parse_arg_count_malformed() {
        assert_eq!(parse_arg_count("invalid"), 0);
        assert_eq!(parse_arg_count("("), 0);
        assert_eq!(parse_arg_count(")"), 0);
        assert_eq!(parse_arg_count("(I"), 0);
    }

    #[test]
    fn parse_arg_types_primitives() {
        assert_eq!(parse_arg_types("(I)V"), vec!['I']);
        assert_eq!(parse_arg_types("(IZB)V"), vec!['I', 'Z', 'B']);
        assert_eq!(parse_arg_types("()V"), Vec::<char>::new());
    }

    #[test]
    fn parse_arg_types_object_and_array() {
        assert_eq!(parse_arg_types("(Ljava/lang/String;)V"), vec!['L']);
        assert_eq!(parse_arg_types("([I)V"), vec!['[']);
        assert_eq!(parse_arg_types("([Ljava/lang/String;)V"), vec!['[']);
        assert_eq!(
            parse_arg_types("(ILjava/lang/String;[I)V"),
            vec!['I', 'L', '[']
        );
    }

    #[test]
    fn parse_arg_types_malformed() {
        assert_eq!(parse_arg_types("invalid"), vec![]);
        assert_eq!(parse_arg_types("("), vec![]);
        assert_eq!(parse_arg_types(")"), vec![]);
        assert_eq!(parse_arg_types("(I"), vec![]);
    }

    // ===========================================================================
    // Helper functions: default_slot_for_descriptor
    // ===========================================================================

    #[test]
    fn default_slot_for_descriptor_long_is_long_zero() {
        assert_eq!(default_slot_for_descriptor("J"), Slot::Long(0));
    }

    #[test]
    fn default_slot_for_descriptor_float_is_float_zero() {
        assert!(matches!(default_slot_for_descriptor("F"), Slot::Float(v) if v == 0.0));
    }

    #[test]
    fn default_slot_for_descriptor_double_is_double_zero() {
        assert!(matches!(default_slot_for_descriptor("D"), Slot::Double(v) if v == 0.0));
    }

    #[test]
    fn default_slot_for_descriptor_reference_types_are_null() {
        assert!(matches!(
            default_slot_for_descriptor("Ljava/lang/String;"),
            Slot::Reference(None)
        ));
        assert!(matches!(
            default_slot_for_descriptor("[I"),
            Slot::Reference(None)
        ));
    }

    #[test]
    fn default_slot_for_descriptor_int_and_others_are_int_zero() {
        assert_eq!(default_slot_for_descriptor("I"), Slot::Int(0));
        assert_eq!(default_slot_for_descriptor("Z"), Slot::Int(0));
        assert_eq!(default_slot_for_descriptor("B"), Slot::Int(0));
    }

    // ===========================================================================
    // Helper function: resolve_cp_string
    // ===========================================================================

    #[test]
    fn resolve_cp_string_from_string_entry() {
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Utf8("hello".to_string())),
            Some(CpEntry::String {
                string_index: CpIndex(1),
            }),
        ];
        assert_eq!(resolve_cp_string(&cp, 2).unwrap(), "hello");
    }

    #[test]
    fn resolve_cp_string_from_utf8_entry() {
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Utf8("world".to_string())),
            Some(CpEntry::String {
                string_index: CpIndex(1),
            }),
        ];
        assert_eq!(resolve_cp_string(&cp, 1).unwrap(), "world");
    }

    #[test]
    fn resolve_cp_string_missing_utf8_raises_error() {
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            None, // missing Utf8
            Some(CpEntry::String {
                string_index: CpIndex(1),
            }),
        ];
        assert!(resolve_cp_string(&cp, 2).is_err());
    }

    #[test]
    fn resolve_cp_string_out_of_bounds_raises_error() {
        let cp: Vec<Option<CpEntry>> = vec![None];
        assert!(resolve_cp_string(&cp, 99).is_err());
    }

    // ===========================================================================
    // Helper function: is_assignable_from — interface walk
    // ===========================================================================

    fn make_simple_loader() -> duke_loader::DirectoryLoader {
        duke_loader::DirectoryLoader::new(std::path::Path::new("."))
    }

    #[test]
    fn is_assignable_from_same_class_is_true() {
        let mut registry = ClassRegistry::new();
        let loader = make_simple_loader();
        assert!(is_assignable_from(&mut registry, &loader, "Foo", "Foo"));
    }

    #[test]
    fn is_assignable_from_object_is_always_true() {
        let mut registry = ClassRegistry::new();
        let loader = make_simple_loader();
        assert!(is_assignable_from(
            &mut registry,
            &loader,
            "Anything",
            "java/lang/Object"
        ));
    }

    #[test]
    fn is_assignable_from_via_direct_interface_is_true() {
        let mut registry = ClassRegistry::new();
        registry.register(ClassContext {
            class_name: "MyClass".to_string(),
            super_class: None,
            interfaces: vec!["MyInterface".to_string()],
            constant_pool: vec![],
            methods: vec![],
            fields: vec![],
            static_fields: vec![],
            instance_field_count: 0,
            bootstrap_methods: vec![],
        });
        let loader = make_simple_loader();
        assert!(is_assignable_from(
            &mut registry,
            &loader,
            "MyClass",
            "MyInterface"
        ));
    }

    #[test]
    fn is_assignable_from_unrelated_class_is_false() {
        let mut registry = ClassRegistry::new();
        registry.register(ClassContext {
            class_name: "MyClass".to_string(),
            super_class: None,
            interfaces: vec!["InterfaceA".to_string()],
            constant_pool: vec![],
            methods: vec![],
            fields: vec![],
            static_fields: vec![],
            instance_field_count: 0,
            bootstrap_methods: vec![],
        });
        let loader = make_simple_loader();
        assert!(!is_assignable_from(
            &mut registry,
            &loader,
            "MyClass",
            "InterfaceB"
        ));
    }

    // ===========================================================================
    // Helper function: find_exception_handler — end_pc boundary
    // ===========================================================================

    #[test]
    fn find_exception_handler_catches_within_range() {
        let table = vec![ExceptionEntry {
            start_pc: 0,
            end_pc: 10,
            handler_pc: 20,
            catch_type: None, // catch-all
        }];
        let mut registry = ClassRegistry::new();
        let loader = make_simple_loader();
        assert_eq!(
            find_exception_handler(&table, 5, "java/lang/Exception", &mut registry, &loader),
            Some(20)
        );
    }

    #[test]
    fn find_exception_handler_excludes_at_end_pc() {
        // end_pc is exclusive per JVM spec
        let table = vec![ExceptionEntry {
            start_pc: 0,
            end_pc: 10,
            handler_pc: 20,
            catch_type: None,
        }];
        let mut registry = ClassRegistry::new();
        let loader = make_simple_loader();
        assert_eq!(
            find_exception_handler(&table, 10, "java/lang/Exception", &mut registry, &loader),
            None
        );
        assert_eq!(
            find_exception_handler(&table, 9, "java/lang/Exception", &mut registry, &loader),
            Some(20)
        );
    }

    // ===========================================================================
    // Helper function: field_slot_idx — single and hierarchical classes
    // ===========================================================================

    fn make_two_class_registry() -> ClassRegistry {
        let mut registry = ClassRegistry::new();
        registry.register(ClassContext {
            class_name: "Base".to_string(),
            super_class: None,
            interfaces: vec![],
            constant_pool: vec![],
            methods: vec![],
            fields: vec![
                FieldEntry {
                    name: "x".to_string(),
                    descriptor: "I".to_string(),
                    is_static: false,
                },
                FieldEntry {
                    name: "y".to_string(),
                    descriptor: "I".to_string(),
                    is_static: false,
                },
            ],
            static_fields: vec![],
            instance_field_count: 2,
            bootstrap_methods: vec![],
        });
        registry.register(ClassContext {
            class_name: "Child".to_string(),
            super_class: Some("Base".to_string()),
            interfaces: vec![],
            constant_pool: vec![],
            methods: vec![],
            fields: vec![FieldEntry {
                name: "z".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            }],
            static_fields: vec![],
            instance_field_count: 1,
            bootstrap_methods: vec![],
        });
        registry
    }

    #[test]
    fn field_slot_idx_finds_first_field_at_slot_0() {
        let registry = make_two_class_registry();
        assert_eq!(field_slot_idx(&registry, "Base", "x").unwrap(), 0);
    }

    #[test]
    fn field_slot_idx_finds_second_field_at_slot_1() {
        let registry = make_two_class_registry();
        assert_eq!(field_slot_idx(&registry, "Base", "y").unwrap(), 1);
    }

    #[test]
    fn field_slot_idx_finds_child_field_after_parent_fields() {
        let registry = make_two_class_registry();
        // Base has 2 fields (x at 0, y at 1); Child adds z → slot 2
        assert_eq!(field_slot_idx(&registry, "Child", "z").unwrap(), 2);
    }

    #[test]
    fn field_slot_idx_missing_field_raises_error() {
        let registry = make_two_class_registry();
        assert!(field_slot_idx(&registry, "Base", "nonexistent").is_err());
    }

    // ===========================================================================
    // native_sb_init_string: null arg falls back to empty string
    // ===========================================================================

    #[test]
    fn native_sb_init_string_null_arg_gives_empty_buffer() {
        let mut heap = duke_gc::Heap::new();
        let this_ref = heap.allocate("java/lang/StringBuilder".to_string(), 1);
        let mut out: Vec<u8> = Vec::new();
        native_sb_init_string(
            &[
                Slot::Reference(Some(this_ref)),
                Slot::Reference(None), // null arg
            ],
            &mut heap,
            &mut out,
        )
        .unwrap();
        assert_eq!(
            heap.get(this_ref).unwrap().string_value.as_deref(),
            Some("")
        );
    }

    // ===========================================================================
    // native_sb_append_string: null appends "null" literal
    // ===========================================================================

    #[test]
    fn native_sb_append_string_null_arg_appends_null_literal() {
        let mut heap = duke_gc::Heap::new();
        let this_ref = heap.allocate("java/lang/StringBuilder".to_string(), 1);
        heap.get_mut(this_ref).unwrap().string_value = Some("hi".to_string());
        let mut out: Vec<u8> = Vec::new();
        native_sb_append_string(
            &[Slot::Reference(Some(this_ref)), Slot::Reference(None)],
            &mut heap,
            &mut out,
        )
        .unwrap();
        assert_eq!(
            heap.get(this_ref).unwrap().string_value.as_deref(),
            Some("hinull")
        );
    }

    // ===========================================================================
    // native_sb_append_char: Slot::Int(v) arm
    // ===========================================================================

    #[test]
    fn native_sb_append_char_appends_unicode_char() {
        let mut heap = duke_gc::Heap::new();
        let this_ref = heap.allocate("java/lang/StringBuilder".to_string(), 1);
        heap.get_mut(this_ref).unwrap().string_value = Some(String::new());
        let mut out: Vec<u8> = Vec::new();
        native_sb_append_char(
            &[
                Slot::Reference(Some(this_ref)),
                Slot::Int('A' as i32), // 65
            ],
            &mut heap,
            &mut out,
        )
        .unwrap();
        assert_eq!(
            heap.get(this_ref).unwrap().string_value.as_deref(),
            Some("A")
        );
    }

    // ===========================================================================
    // native_arraylist_get: index arithmetic (idx + 1)
    // ===========================================================================

    #[test]
    fn native_arraylist_get_index_arithmetic_retrieves_correct_element() {
        let mut heap = duke_gc::Heap::new();
        let list_ref = heap.allocate("java/util/ArrayList".to_string(), 4);
        let v0 = heap.allocate_string("first".to_string());
        let v1 = heap.allocate_string("second".to_string());
        let v2 = heap.allocate_string("third".to_string());
        {
            let obj = heap.get_mut(list_ref).unwrap();
            obj.fields[0] = Slot::Int(3); // size
            obj.fields[1] = Slot::Reference(Some(v0));
            obj.fields[2] = Slot::Reference(Some(v1));
            obj.fields[3] = Slot::Reference(Some(v2));
        }
        let mut out: Vec<u8> = Vec::new();
        // get(1) should return fields[2] = v1 (index 1+1=2)
        let result = native_arraylist_get(
            &[Slot::Reference(Some(list_ref)), Slot::Int(1)],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        if let Slot::Reference(Some(r)) = result {
            assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("second"));
        } else {
            panic!("expected reference, got {result:?}");
        }
    }

    #[test]
    fn native_arraylist_get_index_zero_returns_first_element() {
        let mut heap = duke_gc::Heap::new();
        let list_ref = heap.allocate("java/util/ArrayList".to_string(), 2);
        let v0 = heap.allocate_string("alpha".to_string());
        {
            let obj = heap.get_mut(list_ref).unwrap();
            obj.fields[0] = Slot::Int(1);
            obj.fields[1] = Slot::Reference(Some(v0));
        }
        let mut out: Vec<u8> = Vec::new();
        let result = native_arraylist_get(
            &[Slot::Reference(Some(list_ref)), Slot::Int(0)],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        if let Slot::Reference(Some(r)) = result {
            assert_eq!(heap.get(r).unwrap().string_value.as_deref(), Some("alpha"));
        } else {
            panic!("expected reference");
        }
    }

    // ===========================================================================
    // native_arrays_fill_object: reference value arm
    // ===========================================================================

    #[test]
    fn native_arrays_fill_object_fills_with_reference() {
        let mut heap = duke_gc::Heap::new();
        let arr_ref = heap.allocate("[Ljava/lang/Object;".to_string(), 3);
        let val_ref = heap.allocate_string("x".to_string());
        let mut out: Vec<u8> = Vec::new();
        native_arrays_fill_object(
            &[
                Slot::Reference(Some(arr_ref)),
                Slot::Reference(Some(val_ref)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap();
        let fields = &heap.get(arr_ref).unwrap().fields;
        assert!(
            fields
                .iter()
                .all(|s| matches!(s, Slot::Reference(Some(r)) if *r == val_ref))
        );
    }

    // ===========================================================================
    // native_arrays_copyof_int: negative length raises NegativeArraySize
    // ===========================================================================

    #[test]
    fn native_arrays_copyof_int_negative_length_raises_nsa() {
        let mut heap = duke_gc::Heap::new();
        let src = heap.allocate("[I".to_string(), 3);
        let mut out: Vec<u8> = Vec::new();
        let err = native_arrays_copyof_int(
            &[Slot::Reference(Some(src)), Slot::Int(-1)],
            &mut heap,
            &mut out,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::NegativeArraySize { size: -1 }));
    }

    #[test]
    fn native_arrays_copyof_int_truncates_when_shorter() {
        let mut heap = duke_gc::Heap::new();
        let src = heap.allocate("[I".to_string(), 3);
        {
            let obj = heap.get_mut(src).unwrap();
            obj.fields[0] = Slot::Int(10);
            obj.fields[1] = Slot::Int(20);
            obj.fields[2] = Slot::Int(30);
        }
        let mut out: Vec<u8> = Vec::new();
        let result = native_arrays_copyof_int(
            &[Slot::Reference(Some(src)), Slot::Int(2)],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        if let Slot::Reference(Some(dst_ref)) = result {
            let fields = &heap.get(dst_ref).unwrap().fields;
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0], Slot::Int(10));
            assert_eq!(fields[1], Slot::Int(20));
        } else {
            panic!("expected reference");
        }
    }

    // ===========================================================================
    // native_arrays_copyof_object: negative length raises NegativeArraySize,
    //   and extending with null Reference
    // ===========================================================================

    #[test]
    fn native_arrays_copyof_object_negative_length_raises_nsa() {
        let mut heap = duke_gc::Heap::new();
        let src = heap.allocate("[Ljava/lang/Object;".to_string(), 1);
        let mut out: Vec<u8> = Vec::new();
        let err = native_arrays_copyof_object(
            &[Slot::Reference(Some(src)), Slot::Int(-2)],
            &mut heap,
            &mut out,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::NegativeArraySize { size: -2 }));
    }

    #[test]
    fn native_arrays_copyof_object_extends_with_null() {
        let mut heap = duke_gc::Heap::new();
        let v = heap.allocate_string("item".to_string());
        let src = heap.allocate("[Ljava/lang/Object;".to_string(), 1);
        heap.get_mut(src).unwrap().fields[0] = Slot::Reference(Some(v));
        let mut out: Vec<u8> = Vec::new();
        let result = native_arrays_copyof_object(
            &[Slot::Reference(Some(src)), Slot::Int(3)],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        if let Slot::Reference(Some(dst_ref)) = result {
            let fields = &heap.get(dst_ref).unwrap().fields;
            assert_eq!(fields.len(), 3);
            assert!(matches!(fields[0], Slot::Reference(Some(_))));
            assert!(matches!(fields[1], Slot::Reference(None)));
            assert!(matches!(fields[2], Slot::Reference(None)));
        } else {
            panic!("expected reference");
        }
    }

    // ===========================================================================
    // slots_equal: null-null, null-nonnull, string equality, class equality
    // ===========================================================================

    #[test]
    fn slots_equal_null_null_is_true() {
        let heap = duke_gc::Heap::new();
        assert!(slots_equal(
            &Slot::Reference(None),
            &Slot::Reference(None),
            &heap
        ));
    }

    #[test]
    fn slots_equal_null_nonnull_is_false() {
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate_string("x".to_string());
        assert!(!slots_equal(
            &Slot::Reference(None),
            &Slot::Reference(Some(r)),
            &heap
        ));
        assert!(!slots_equal(
            &Slot::Reference(Some(r)),
            &Slot::Reference(None),
            &heap
        ));
    }

    #[test]
    fn slots_equal_strings_compared_by_value() {
        let mut heap = duke_gc::Heap::new();
        let r1 = heap.allocate_string("hello".to_string());
        let r2 = heap.allocate_string("hello".to_string()); // different ref, same value
        let r3 = heap.allocate_string("world".to_string());
        assert!(slots_equal(
            &Slot::Reference(Some(r1)),
            &Slot::Reference(Some(r2)),
            &heap
        ));
        assert!(!slots_equal(
            &Slot::Reference(Some(r1)),
            &Slot::Reference(Some(r3)),
            &heap
        ));
    }

    #[test]
    fn slots_equal_same_class_same_first_field_is_true() {
        let mut heap = duke_gc::Heap::new();
        let r1 = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(r1).unwrap().fields[0] = Slot::Int(42);
        let r2 = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(r2).unwrap().fields[0] = Slot::Int(42);
        assert!(slots_equal(
            &Slot::Reference(Some(r1)),
            &Slot::Reference(Some(r2)),
            &heap
        ));
    }

    #[test]
    fn slots_equal_different_classes_is_false() {
        let mut heap = duke_gc::Heap::new();
        let r1 = heap.allocate("java/lang/Integer".to_string(), 1);
        heap.get_mut(r1).unwrap().fields[0] = Slot::Int(1);
        let r2 = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(r2).unwrap().fields[0] = Slot::Int(1);
        assert!(!slots_equal(
            &Slot::Reference(Some(r1)),
            &Slot::Reference(Some(r2)),
            &heap
        ));
    }

    // ===========================================================================
    // native_hashmap_put/get/contains_key/remove/get_or_default with 2 entries
    // (exercises the i += 2 loop arithmetic)
    // ===========================================================================

    fn make_hashmap_with_two_string_entries() -> (duke_gc::Heap, u64, u64, u64, u64, u64) {
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let hm_ref = heap.allocate("java/util/HashMap".to_string(), 1);
        native_hashmap_init(&[Slot::Reference(Some(hm_ref))], &mut heap, &mut out).unwrap();
        let k1 = heap.allocate_string("key1".to_string());
        let v1 = heap.allocate_string("val1".to_string());
        let k2 = heap.allocate_string("key2".to_string());
        let v2 = heap.allocate_string("val2".to_string());
        native_hashmap_put(
            &[
                Slot::Reference(Some(hm_ref)),
                Slot::Reference(Some(k1)),
                Slot::Reference(Some(v1)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap();
        native_hashmap_put(
            &[
                Slot::Reference(Some(hm_ref)),
                Slot::Reference(Some(k2)),
                Slot::Reference(Some(v2)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap();
        (heap, hm_ref, k1, v1, k2, v2)
    }

    #[test]
    fn native_hashmap_get_finds_second_entry() {
        let (mut heap, hm_ref, _k1, _v1, k2, v2) = make_hashmap_with_two_string_entries();
        let mut out: Vec<u8> = Vec::new();
        let result = native_hashmap_get(
            &[Slot::Reference(Some(hm_ref)), Slot::Reference(Some(k2))],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        if let Slot::Reference(Some(r)) = result {
            assert_eq!(
                heap.get(r).unwrap().string_value,
                heap.get(v2).unwrap().string_value
            );
        } else {
            panic!("expected reference, got {result:?}");
        }
    }

    #[test]
    fn native_hashmap_get_finds_first_entry() {
        let (mut heap, hm_ref, k1, v1, _k2, _v2) = make_hashmap_with_two_string_entries();
        let mut out: Vec<u8> = Vec::new();
        let result = native_hashmap_get(
            &[Slot::Reference(Some(hm_ref)), Slot::Reference(Some(k1))],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        if let Slot::Reference(Some(r)) = result {
            assert_eq!(
                heap.get(r).unwrap().string_value,
                heap.get(v1).unwrap().string_value
            );
        } else {
            panic!("expected reference, got {result:?}");
        }
    }

    #[test]
    fn native_hashmap_contains_key_finds_second_entry() {
        let (mut heap, hm_ref, _k1, _v1, k2, _v2) = make_hashmap_with_two_string_entries();
        let mut out: Vec<u8> = Vec::new();
        let r = native_hashmap_contains_key(
            &[Slot::Reference(Some(hm_ref)), Slot::Reference(Some(k2))],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn native_hashmap_contains_key_absent_key_is_false() {
        let (mut heap, hm_ref, _k1, _v1, _k2, _v2) = make_hashmap_with_two_string_entries();
        let other_key = heap.allocate_string("absent".to_string());
        let mut out: Vec<u8> = Vec::new();
        let r = native_hashmap_contains_key(
            &[
                Slot::Reference(Some(hm_ref)),
                Slot::Reference(Some(other_key)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn native_hashmap_remove_second_entry_decrements_size() {
        let (mut heap, hm_ref, _k1, _v1, k2, v2) = make_hashmap_with_two_string_entries();
        let mut out: Vec<u8> = Vec::new();
        let old = native_hashmap_remove(
            &[Slot::Reference(Some(hm_ref)), Slot::Reference(Some(k2))],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        if let Slot::Reference(Some(r)) = old {
            assert_eq!(
                heap.get(r).unwrap().string_value,
                heap.get(v2).unwrap().string_value
            );
        } else {
            panic!("expected reference, got {old:?}");
        }
        // Size should now be 1
        let size = native_hashmap_size(&[Slot::Reference(Some(hm_ref))], &mut heap, &mut out)
            .unwrap()
            .unwrap();
        assert_eq!(size, Slot::Int(1));
    }

    #[test]
    fn native_hashmap_remove_first_entry_leaves_second_findable() {
        let (mut heap, hm_ref, k1, _v1, k2, v2) = make_hashmap_with_two_string_entries();
        let mut out: Vec<u8> = Vec::new();
        // Remove first entry
        native_hashmap_remove(
            &[Slot::Reference(Some(hm_ref)), Slot::Reference(Some(k1))],
            &mut heap,
            &mut out,
        )
        .unwrap();
        // Second entry should still be findable
        let r = native_hashmap_get(
            &[Slot::Reference(Some(hm_ref)), Slot::Reference(Some(k2))],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        if let Slot::Reference(Some(found)) = r {
            assert_eq!(
                heap.get(found).unwrap().string_value,
                heap.get(v2).unwrap().string_value
            );
        } else {
            panic!("expected reference after remove");
        }
    }

    #[test]
    fn native_hashmap_get_or_default_returns_second_entry_value() {
        let (mut heap, hm_ref, _k1, _v1, k2, v2) = make_hashmap_with_two_string_entries();
        let def = heap.allocate_string("default".to_string());
        let mut out: Vec<u8> = Vec::new();
        let result = native_hashmap_get_or_default(
            &[
                Slot::Reference(Some(hm_ref)),
                Slot::Reference(Some(k2)),
                Slot::Reference(Some(def)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        if let Slot::Reference(Some(r)) = result {
            assert_ne!(
                heap.get(r).unwrap().string_value,
                heap.get(def).unwrap().string_value
            );
            assert_eq!(
                heap.get(r).unwrap().string_value,
                heap.get(v2).unwrap().string_value
            );
        } else {
            panic!("expected reference");
        }
    }

    #[test]
    fn native_hashmap_get_or_default_returns_default_when_absent() {
        let (mut heap, hm_ref, _k1, _v1, _k2, _v2) = make_hashmap_with_two_string_entries();
        let missing_key = heap.allocate_string("missing".to_string());
        let def = heap.allocate_string("default_val".to_string());
        let mut out: Vec<u8> = Vec::new();
        let result = native_hashmap_get_or_default(
            &[
                Slot::Reference(Some(hm_ref)),
                Slot::Reference(Some(missing_key)),
                Slot::Reference(Some(def)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        if let Slot::Reference(Some(r)) = result {
            assert_eq!(r, def);
        } else {
            panic!("expected reference");
        }
    }

    #[test]
    fn native_hashmap_put_update_existing_key_returns_old_value() {
        let (mut heap, hm_ref, k1, v1, _k2, _v2) = make_hashmap_with_two_string_entries();
        let new_val = heap.allocate_string("new_val1".to_string());
        let mut out: Vec<u8> = Vec::new();
        // Update k1 → should return old value v1
        let old = native_hashmap_put(
            &[
                Slot::Reference(Some(hm_ref)),
                Slot::Reference(Some(k1)),
                Slot::Reference(Some(new_val)),
            ],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        if let Slot::Reference(Some(r)) = old {
            assert_eq!(
                heap.get(r).unwrap().string_value,
                heap.get(v1).unwrap().string_value
            );
        } else {
            panic!("expected old value reference, got {old:?}");
        }
    }

    // ===========================================================================
    // execute_class() synthetic tests — covers execute_class opcode paths
    // (Distinct from execute() tests; kills mutants in execute_class body.)
    // ===========================================================================

    /// Run an instruction stream directly through `execute_class()` using a
    /// synthetic `ClassContext`, killing mutants in the `execute_class()` switch body.
    #[allow(clippy::needless_pass_by_value)]
    fn execute_class_synthetic(
        instructions: Vec<(usize, Instruction)>,
        args: Vec<Slot>,
        max_stack: u16,
        max_locals: u16,
        descriptor: &str,
    ) -> VmResult<Option<Slot>> {
        use std::sync::Arc;
        let pc_to_idx: std::collections::HashMap<usize, usize> = instructions
            .iter()
            .enumerate()
            .map(|(idx, (pc, _))| (*pc, idx))
            .collect();
        let method = MethodEntry {
            name: "syntest".to_string(),
            descriptor: descriptor.to_string(),
            instructions: instructions.into(),
            max_stack,
            max_locals,
            exception_table: vec![],
            pc_to_idx: Arc::new(pc_to_idx),
        };
        let ctx = ClassContext {
            class_name: "SynTest".to_string(),
            super_class: None,
            interfaces: vec![],
            constant_pool: vec![None],
            methods: vec![method],
            fields: vec![],
            static_fields: vec![],
            instance_field_count: 0,
            bootstrap_methods: vec![],
        };
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = make_simple_loader();
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "SynTest",
            "syntest",
            descriptor,
            &args,
        )
    }

    // ---- execute_class: Iushr ----

    #[test]
    fn ec_iushr_masks_shift_count() {
        // -8 >>> 2: mask 2 & 0x1F = 2; if | instead: 2|31=31 → result differs
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Iload1),
                (2, Instruction::Iushr),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Int(-8), Slot::Int(2)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        // (-8u32) >> 2 = 0x3FFFFFFE = 1073741822
        assert_eq!(r, Slot::Int(1_073_741_822));
    }

    // ---- execute_class: Iand / Ior / Ixor ----

    #[test]
    fn ec_iand_selects_common_bits() {
        // 0b11110000 & 0b10101010 = 0b10100000 = 160
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Iload1),
                (2, Instruction::Iand),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Int(0b1111_0000), Slot::Int(0b1010_1010)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0b1010_0000)); // 160
    }

    #[test]
    fn ec_ior_combines_bits() {
        // 12 | 10 = 14
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Iload1),
                (2, Instruction::Ior),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Int(12), Slot::Int(10)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(14));
    }

    #[test]
    fn ec_ixor_flips_differing_bits() {
        // 5 ^ 3 = 6; if | then 7; if & then 1
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Iload1),
                (2, Instruction::Ixor),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Int(5), Slot::Int(3)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(6));
    }

    // ---- execute_class: Ldiv / Lrem division-by-zero checks ----

    #[test]
    fn ec_ldiv_normal_returns_quotient() {
        // 10L / 3L = 3L; if b==0 check becomes !=, divides when b=3 (not 0) → errors instead
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Lload0),
                (1, Instruction::Lload1),
                (2, Instruction::Ldiv),
                (3, Instruction::Lreturn),
            ],
            vec![Slot::Long(10), Slot::Long(3)],
            4,
            2,
            "()J",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(3));
    }

    #[test]
    fn ec_lrem_normal_returns_remainder() {
        // 10L % 3L = 1L; same kill logic as ldiv
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Lload0),
                (1, Instruction::Lload1),
                (2, Instruction::Lrem),
                (3, Instruction::Lreturn),
            ],
            vec![Slot::Long(10), Slot::Long(3)],
            4,
            2,
            "()J",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(1));
    }

    // ---- execute_class: Lshl / Lshr / Lushr masking ----

    #[test]
    fn ec_lshl_masks_shift_by_63_not_127() {
        // s=65, 65 & 0x3F = 1; if | then 65|63=127 → wrapping_shl(127) ≠ wrapping_shl(1)
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Lload0),
                (1, Instruction::Iload1),
                (2, Instruction::Lshl),
                (3, Instruction::Lreturn),
            ],
            vec![Slot::Long(1), Slot::Int(65)],
            4,
            2,
            "()J",
        )
        .unwrap()
        .unwrap();
        // 1L << 1 = 2
        assert_eq!(r, Slot::Long(2));
    }

    #[test]
    fn ec_lshr_masks_shift_count() {
        // -8L >> 65; 65 & 63 = 1 → -8 >> 1 = -4; if | then 65|63=127 → 127%64=63 → -8>>63=-1
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Lload0),
                (1, Instruction::Iload1),
                (2, Instruction::Lshr),
                (3, Instruction::Lreturn),
            ],
            vec![Slot::Long(-8), Slot::Int(65)],
            4,
            2,
            "()J",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(-4));
    }

    #[test]
    fn ec_lushr_right_shift_direction() {
        // i64::MIN >>> 1 = large positive; if << instead: 0
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Lload0),
                (1, Instruction::Iload1),
                (2, Instruction::Lushr),
                (3, Instruction::Lreturn),
            ],
            vec![Slot::Long(i64::MIN), Slot::Int(1)],
            4,
            2,
            "()J",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(4_611_686_018_427_387_904));
    }

    #[test]
    fn ec_lushr_masks_shift_count() {
        // i64::MIN >>> 65; 65 & 63 = 1 → same as >>>1; if | then 65|63=127 → 127%64=63 → 1
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Lload0),
                (1, Instruction::Iload1),
                (2, Instruction::Lushr),
                (3, Instruction::Lreturn),
            ],
            vec![Slot::Long(i64::MIN), Slot::Int(65)],
            4,
            2,
            "()J",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(4_611_686_018_427_387_904));
    }

    // ---- execute_class: Land / Lor / Lxor ----

    #[test]
    fn ec_land_selects_common_bits() {
        // 0xF0F0F0F0L & 0x0F0F0F0FL = 0 (no common bits)
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Lload0),
                (1, Instruction::Lload1),
                (2, Instruction::Land),
                (3, Instruction::Lreturn),
            ],
            vec![Slot::Long(0xF0F0_F0F0), Slot::Long(0x0F0F_0F0F)],
            4,
            2,
            "()J",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(0));
    }

    #[test]
    fn ec_lor_combines_bits() {
        // 0b1100L | 0b0110L = 0b1110 = 14
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Lload0),
                (1, Instruction::Lload1),
                (2, Instruction::Lor),
                (3, Instruction::Lreturn),
            ],
            vec![Slot::Long(0b1100), Slot::Long(0b0110)],
            4,
            2,
            "()J",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(14));
    }

    #[test]
    fn ec_lxor_flips_differing_bits() {
        // 0b1010L ^ 0b1010L = 0; | or & both give 0b1010=10
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Lload0),
                (1, Instruction::Lload1),
                (2, Instruction::Lxor),
                (3, Instruction::Lreturn),
            ],
            vec![Slot::Long(0b1010), Slot::Long(0b1010)],
            4,
            2,
            "()J",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(0));
    }

    // ---- execute_class: Lcmp (-1 arm) ----

    #[test]
    fn ec_lcmp_less_than_returns_minus_one() {
        // lcmp(2, 5) → -1; delete - mutant returns 1
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Lload0),
                (1, Instruction::Lload1),
                (2, Instruction::Lcmp),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Long(2), Slot::Long(5)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(-1));
    }

    #[test]
    fn ec_lcmp_equal_returns_zero() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Lload0),
                (1, Instruction::Lload1),
                (2, Instruction::Lcmp),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Long(7), Slot::Long(7)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn ec_lcmp_greater_than_returns_one() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Lload0),
                (1, Instruction::Lload1),
                (2, Instruction::Lcmp),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Long(10), Slot::Long(3)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    // ---- execute_class: Float arithmetic ----

    #[test]
    fn ec_fadd_adds_floats() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fadd),
                (3, Instruction::Freturn),
            ],
            vec![Slot::Float(2.0), Slot::Float(3.0)],
            4,
            2,
            "()F",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Float(5.0));
    }

    #[test]
    fn ec_fsub_subtracts_floats() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fsub),
                (3, Instruction::Freturn),
            ],
            vec![Slot::Float(7.0), Slot::Float(3.0)],
            4,
            2,
            "()F",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Float(4.0));
    }

    #[test]
    fn ec_fmul_multiplies_floats() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fmul),
                (3, Instruction::Freturn),
            ],
            vec![Slot::Float(4.0), Slot::Float(3.0)],
            4,
            2,
            "()F",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Float(12.0));
    }

    #[test]
    fn ec_fdiv_divides_floats() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fdiv),
                (3, Instruction::Freturn),
            ],
            vec![Slot::Float(10.0), Slot::Float(4.0)],
            4,
            2,
            "()F",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Float(2.5));
    }

    #[test]
    fn ec_frem_float_remainder() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Frem),
                (3, Instruction::Freturn),
            ],
            vec![Slot::Float(10.0), Slot::Float(3.0)],
            4,
            2,
            "()F",
        )
        .unwrap()
        .unwrap();
        if let Slot::Float(v) = r {
            assert!((v - 1.0_f32).abs() < 0.001);
        } else {
            panic!("{r:?}");
        }
    }

    #[test]
    fn ec_fneg_negates_float() {
        // -(-5.0) = 5.0; if delete - mutant: -5.0 != 5.0
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Fload0),
                (1, Instruction::Fneg),
                (2, Instruction::Freturn),
            ],
            vec![Slot::Float(-5.0)],
            4,
            1,
            "()F",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Float(5.0));
    }

    // ---- execute_class: Fcmpl / Fcmpg ----

    #[test]
    fn ec_fcmpl_greater_returns_1() {
        // a > b → 1; if > mutated to < then returns -1
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fcmpl),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Float(3.0), Slot::Float(2.0)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn ec_fcmpl_less_returns_minus_1() {
        // a < b → -1; delete - mutant returns 1
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fcmpl),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Float(2.0), Slot::Float(3.0)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(-1));
    }

    #[test]
    fn ec_fcmpl_nan_returns_minus_1() {
        // NaN case for Fcmpl → -1; delete - mutant at 6033 returns 1
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fcmpl),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Float(f32::NAN), Slot::Float(0.0)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(-1));
    }

    #[test]
    fn ec_fcmpg_nan_returns_1() {
        // NaN case for Fcmpg → +1; verifies Fcmpg differs from Fcmpl for NaN
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Fload0),
                (1, Instruction::Fload1),
                (2, Instruction::Fcmpg),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Float(f32::NAN), Slot::Float(0.0)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    // ---- execute_class: Double arithmetic ----

    #[test]
    fn ec_dadd_adds_doubles() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dadd),
                (3, Instruction::Dreturn),
            ],
            vec![Slot::Double(2.0), Slot::Double(3.0)],
            4,
            2,
            "()D",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Double(5.0));
    }

    #[test]
    fn ec_dsub_subtracts_doubles() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dsub),
                (3, Instruction::Dreturn),
            ],
            vec![Slot::Double(7.0), Slot::Double(3.0)],
            4,
            2,
            "()D",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Double(4.0));
    }

    #[test]
    fn ec_dmul_multiplies_doubles() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dmul),
                (3, Instruction::Dreturn),
            ],
            vec![Slot::Double(4.0), Slot::Double(3.0)],
            4,
            2,
            "()D",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Double(12.0));
    }

    #[test]
    fn ec_ddiv_divides_doubles() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Ddiv),
                (3, Instruction::Dreturn),
            ],
            vec![Slot::Double(10.0), Slot::Double(4.0)],
            4,
            2,
            "()D",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Double(2.5));
    }

    #[test]
    fn ec_drem_double_remainder() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Drem),
                (3, Instruction::Dreturn),
            ],
            vec![Slot::Double(10.0), Slot::Double(3.0)],
            4,
            2,
            "()D",
        )
        .unwrap()
        .unwrap();
        if let Slot::Double(v) = r {
            assert!((v - 1.0_f64).abs() < 1e-9);
        } else {
            panic!("{r:?}");
        }
    }

    #[test]
    fn ec_dneg_negates_double() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Dload0),
                (1, Instruction::Dneg),
                (2, Instruction::Dreturn),
            ],
            vec![Slot::Double(-5.0)],
            4,
            1,
            "()D",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Double(5.0));
    }

    // ---- execute_class: Dcmpl / Dcmpg ----

    #[test]
    fn ec_dcmpl_greater_returns_1() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dcmpl),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Double(3.0), Slot::Double(2.0)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn ec_dcmpl_less_returns_minus_1() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dcmpl),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Double(2.0), Slot::Double(3.0)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(-1));
    }

    #[test]
    fn ec_dcmpl_nan_returns_minus_1() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dcmpl),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Double(f64::NAN), Slot::Double(0.0)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(-1));
    }

    #[test]
    fn ec_dcmpg_nan_returns_1() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Dload0),
                (1, Instruction::Dload1),
                (2, Instruction::Dcmpg),
                (3, Instruction::Ireturn),
            ],
            vec![Slot::Double(f64::NAN), Slot::Double(0.0)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    // ---- execute_class: Conditional branches ----
    // Branch layout: (0,push), (1,Ifxx(5))→target=6, (4,Iconst0)→not-taken, (5,Ireturn),
    //                (6,Iconst1)→taken, (7,Ireturn)

    #[test]
    fn ec_iflt_taken_on_negative() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Iflt(5)), // target = 1+5 = 6
                (4, Instruction::Iconst0),
                (5, Instruction::Ireturn),
                (6, Instruction::Iconst1),
                (7, Instruction::Ireturn),
            ],
            vec![Slot::Int(-1)],
            4,
            1,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn ec_iflt_not_taken_on_zero() {
        // zero is not < 0; mutant (< → ==) makes 0==0 → taken → 1 ≠ 0
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Iflt(5)),
                (4, Instruction::Iconst0),
                (5, Instruction::Ireturn),
                (6, Instruction::Iconst1),
                (7, Instruction::Ireturn),
            ],
            vec![Slot::Int(0)],
            4,
            1,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn ec_iflt_not_taken_on_positive() {
        // +1 not < 0; mutant (< → >) makes 1>0 → taken → 1 ≠ 0
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Iflt(5)),
                (4, Instruction::Iconst0),
                (5, Instruction::Ireturn),
                (6, Instruction::Iconst1),
                (7, Instruction::Ireturn),
            ],
            vec![Slot::Int(1)],
            4,
            1,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn ec_ifgt_taken_on_positive() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Ifgt(5)), // target = 6
                (4, Instruction::Iconst0),
                (5, Instruction::Ireturn),
                (6, Instruction::Iconst1),
                (7, Instruction::Ireturn),
            ],
            vec![Slot::Int(1)],
            4,
            1,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn ec_ifgt_not_taken_on_zero() {
        // 0 not > 0; mutant (> → ==): 0==0 → taken → 1 ≠ 0
        // also kills > → >= mutant
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Ifgt(5)),
                (4, Instruction::Iconst0),
                (5, Instruction::Ireturn),
                (6, Instruction::Iconst1),
                (7, Instruction::Ireturn),
            ],
            vec![Slot::Int(0)],
            4,
            1,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn ec_ifgt_not_taken_on_negative() {
        // -1 not > 0; mutant (> → <): -1<0 → taken → 1 ≠ 0
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Ifgt(5)),
                (4, Instruction::Iconst0),
                (5, Instruction::Ireturn),
                (6, Instruction::Iconst1),
                (7, Instruction::Ireturn),
            ],
            vec![Slot::Int(-1)],
            4,
            1,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn ec_ificmpeq_taken_when_equal() {
        // a==b → taken; mutant (== → !=): not taken → 0 ≠ 1
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Iload1),
                (2, Instruction::IfIcmpeq(5)), // target = 2+5 = 7
                (5, Instruction::Iconst0),
                (6, Instruction::Ireturn),
                (7, Instruction::Iconst1),
                (8, Instruction::Ireturn),
            ],
            vec![Slot::Int(3), Slot::Int(3)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn ec_ificmpeq_not_taken_when_unequal() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Iload1),
                (2, Instruction::IfIcmpeq(5)),
                (5, Instruction::Iconst0),
                (6, Instruction::Ireturn),
                (7, Instruction::Iconst1),
                (8, Instruction::Ireturn),
            ],
            vec![Slot::Int(3), Slot::Int(4)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn ec_ificmplt_taken_when_less() {
        // 2 < 5 → taken; mutant (< → ==): 2==5 = false → 0 ≠ 1
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Iload1),
                (2, Instruction::IfIcmplt(5)),
                (5, Instruction::Iconst0),
                (6, Instruction::Ireturn),
                (7, Instruction::Iconst1),
                (8, Instruction::Ireturn),
            ],
            vec![Slot::Int(2), Slot::Int(5)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn ec_ificmplt_not_taken_when_equal() {
        // 3 not < 3 → 0; mutant (< → ==): 3==3=true → 1; mutant (< → <=): 3<=3=true → 1
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Iload1),
                (2, Instruction::IfIcmplt(5)),
                (5, Instruction::Iconst0),
                (6, Instruction::Ireturn),
                (7, Instruction::Iconst1),
                (8, Instruction::Ireturn),
            ],
            vec![Slot::Int(3), Slot::Int(3)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn ec_ificmplt_not_taken_when_greater() {
        // 5 not < 2 → 0; mutant (< → >): 5>2=true → 1
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Iload1),
                (2, Instruction::IfIcmplt(5)),
                (5, Instruction::Iconst0),
                (6, Instruction::Ireturn),
                (7, Instruction::Iconst1),
                (8, Instruction::Ireturn),
            ],
            vec![Slot::Int(5), Slot::Int(2)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn ec_ificmple_taken_when_equal() {
        // 3 <= 3 → taken → 1; mutant (<= → >): 3>3=false → 0
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Iload1),
                (2, Instruction::IfIcmple(5)),
                (5, Instruction::Iconst0),
                (6, Instruction::Ireturn),
                (7, Instruction::Iconst1),
                (8, Instruction::Ireturn),
            ],
            vec![Slot::Int(3), Slot::Int(3)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn ec_ificmple_not_taken_when_greater() {
        // 4 not <= 3 → 0; mutant (<= → >): 4>3=true → 1
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iload0),
                (1, Instruction::Iload1),
                (2, Instruction::IfIcmple(5)),
                (5, Instruction::Iconst0),
                (6, Instruction::Ireturn),
                (7, Instruction::Iconst1),
                (8, Instruction::Ireturn),
            ],
            vec![Slot::Int(4), Slot::Int(3)],
            4,
            2,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    // ---- execute_class: Newarray init for Long/Float/Double ----

    #[test]
    fn ec_newarray_long_default_slot_is_long_zero() {
        // Create long[1]; arm deleted → fields stay Int(0) → Laload TypeMismatch
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iconst1),
                (
                    1,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Long),
                ),
                (3, Instruction::Astore0),
                (4, Instruction::Aload0),
                (5, Instruction::Iconst0),
                (6, Instruction::Laload),
                (7, Instruction::Lreturn),
            ],
            vec![],
            4,
            1,
            "()J",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(0));
    }

    #[test]
    fn ec_newarray_float_default_slot_is_float_zero() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iconst1),
                (
                    1,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Float),
                ),
                (3, Instruction::Astore0),
                (4, Instruction::Aload0),
                (5, Instruction::Iconst0),
                (6, Instruction::Faload),
                (7, Instruction::Freturn),
            ],
            vec![],
            4,
            1,
            "()F",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Float(0.0));
    }

    #[test]
    fn ec_newarray_double_default_slot_is_double_zero() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iconst1),
                (
                    1,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Double),
                ),
                (3, Instruction::Astore0),
                (4, Instruction::Aload0),
                (5, Instruction::Iconst0),
                (6, Instruction::Daload),
                (7, Instruction::Dreturn),
            ],
            vec![],
            4,
            1,
            "()D",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Double(0.0));
    }

    // ---- execute_class: Array bounds — kills || → && and < mutations ----

    /// Create int[3], access valid idx=0 → succeeds.
    /// Kills `idx_val < 0` mutated to `== 0` and `<= 0` (idx=0 would false-trigger).
    #[test]
    fn ec_iaload_valid_idx0_succeeds() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Int),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Iconst0),
                (7, Instruction::Iaload),
                (8, Instruction::Ireturn),
            ],
            vec![],
            4,
            1,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    /// Access valid idx=1 of int[3]; kills `< → >` (1>0=true would wrongly error).
    #[test]
    fn ec_iaload_valid_idx1_succeeds() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Int),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Iconst1),
                (7, Instruction::Iaload),
                (8, Instruction::Ireturn),
            ],
            vec![],
            4,
            1,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    /// Access OOB idx=3 of int[3]; kills `|| → &&` (false && true = no error).
    #[test]
    fn ec_iaload_oob_at_length_errors() {
        let err = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Int),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Bipush(3)), // idx = length = OOB
                (8, Instruction::Iaload),
                (9, Instruction::Ireturn),
            ],
            vec![],
            4,
            1,
            "()I",
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    /// Iastore OOB at length; kills its || → && mutant.
    #[test]
    fn ec_iastore_oob_at_length_errors() {
        let err = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Int),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Bipush(3)), // idx=3 = OOB
                (8, Instruction::Iconst5),   // value
                (9, Instruction::Iastore),
                (10, Instruction::Iconst0),
                (11, Instruction::Ireturn),
            ],
            vec![],
            4,
            1,
            "()I",
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    /// Laload: valid idx=0 succeeds (kills < → == at Laload bounds).
    #[test]
    fn ec_laload_valid_idx0_succeeds() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Long),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Iconst0),
                (7, Instruction::Laload),
                (8, Instruction::Lreturn),
            ],
            vec![],
            4,
            1,
            "()J",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(0));
    }

    /// Laload: valid idx=1 succeeds (kills < → > at Laload bounds).
    #[test]
    fn ec_laload_valid_idx1_succeeds() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Long),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Iconst1),
                (7, Instruction::Laload),
                (8, Instruction::Lreturn),
            ],
            vec![],
            4,
            1,
            "()J",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(0));
    }

    /// Laload OOB kills || → &&.
    #[test]
    fn ec_laload_oob_at_length_errors() {
        let err = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Long),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Bipush(3)),
                (8, Instruction::Laload),
                (9, Instruction::Lreturn),
            ],
            vec![],
            4,
            1,
            "()J",
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    /// Lastore OOB kills || → &&.
    #[test]
    fn ec_lastore_oob_at_length_errors() {
        let err = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Long),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Bipush(3)), // idx=3 OOB
                (8, Instruction::Lconst0),
                (9, Instruction::Lastore),
                (10, Instruction::Iconst0),
                (11, Instruction::Ireturn),
            ],
            vec![],
            4,
            1,
            "()I",
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    /// Faload valid idx=0 and idx=1.
    #[test]
    fn ec_faload_valid_idx0_succeeds() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Float),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Iconst0),
                (7, Instruction::Faload),
                (8, Instruction::Freturn),
            ],
            vec![],
            4,
            1,
            "()F",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Float(0.0));
    }

    #[test]
    fn ec_faload_oob_at_length_errors() {
        let err = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Float),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Bipush(3)),
                (8, Instruction::Faload),
                (9, Instruction::Freturn),
            ],
            vec![],
            4,
            1,
            "()F",
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    #[test]
    fn ec_fastore_oob_at_length_errors() {
        let err = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Float),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Bipush(3)), // idx=3 OOB
                (8, Instruction::Fconst0),
                (9, Instruction::Fastore),
                (10, Instruction::Iconst0),
                (11, Instruction::Ireturn),
            ],
            vec![],
            4,
            1,
            "()I",
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    #[test]
    fn ec_daload_valid_idx0_succeeds() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Double),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Iconst0),
                (7, Instruction::Daload),
                (8, Instruction::Dreturn),
            ],
            vec![],
            4,
            1,
            "()D",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Double(0.0));
    }

    #[test]
    fn ec_daload_valid_idx1_succeeds() {
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Double),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Iconst1),
                (7, Instruction::Daload),
                (8, Instruction::Dreturn),
            ],
            vec![],
            4,
            1,
            "()D",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Double(0.0));
    }

    #[test]
    fn ec_daload_oob_at_length_errors() {
        let err = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Double),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Bipush(3)),
                (8, Instruction::Daload),
                (9, Instruction::Dreturn),
            ],
            vec![],
            4,
            1,
            "()D",
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    #[test]
    fn ec_dastore_oob_at_length_errors() {
        let err = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(3)),
                (
                    2,
                    Instruction::Newarray(duke_bytecode::instruction::ArrayType::Double),
                ),
                (4, Instruction::Astore0),
                (5, Instruction::Aload0),
                (6, Instruction::Bipush(3)), // idx=3 OOB
                (8, Instruction::Dconst0),
                (9, Instruction::Dastore),
                (10, Instruction::Iconst0),
                (11, Instruction::Ireturn),
            ],
            vec![],
            4,
            1,
            "()I",
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- run_class_int on Arithmetic.class via execute_class() ----
    // These tests exercise the same operations as run_static_int but via execute_class,
    // killing mutants in the execute_class() opcode body.

    #[test]
    fn ec_arithmetic_bitwise_xor() {
        // 5 ^ 3 = 6; via execute_class() kills Ixor mutants in execute_class
        assert_eq!(
            run_class_int("Arithmetic.class", "bitwiseXor", "(II)I", vec![5, 3]),
            6
        );
    }

    #[test]
    fn ec_arithmetic_bitwise_and() {
        assert_eq!(
            run_class_int(
                "Arithmetic.class",
                "bitwiseAnd",
                "(II)I",
                vec![0b1111, 0b1010]
            ),
            0b1010
        );
    }

    #[test]
    fn ec_arithmetic_bitwise_or() {
        assert_eq!(
            run_class_int(
                "Arithmetic.class",
                "bitwiseOr",
                "(II)I",
                vec![0b1111, 0b1010]
            ),
            0b1111
        );
    }

    #[test]
    fn ec_arithmetic_abs_negative() {
        // abs(-5) = 5; exercises conditional branch in execute_class
        assert_eq!(
            run_class_int("Arithmetic.class", "abs", "(I)I", vec![-5]),
            5
        );
    }

    #[test]
    fn ec_arithmetic_abs_positive() {
        assert_eq!(run_class_int("Arithmetic.class", "abs", "(I)I", vec![5]), 5);
    }

    #[test]
    fn ec_arithmetic_max_first_greater() {
        assert_eq!(
            run_class_int("Arithmetic.class", "max", "(II)I", vec![7, 3]),
            7
        );
    }

    #[test]
    fn ec_arithmetic_max_second_greater() {
        assert_eq!(
            run_class_int("Arithmetic.class", "max", "(II)I", vec![3, 7]),
            7
        );
    }

    #[test]
    fn ec_arithmetic_clamp_in_range() {
        assert_eq!(
            run_class_int("Arithmetic.class", "clamp", "(III)I", vec![5, 1, 10]),
            5
        );
    }

    #[test]
    fn ec_arithmetic_clamp_below_lo() {
        assert_eq!(
            run_class_int("Arithmetic.class", "clamp", "(III)I", vec![0, 1, 10]),
            1
        );
    }

    #[test]
    fn ec_arithmetic_clamp_above_hi() {
        assert_eq!(
            run_class_int("Arithmetic.class", "clamp", "(III)I", vec![15, 1, 10]),
            10
        );
    }

    #[test]
    fn ec_arithmetic_fibonacci_10() {
        // Exercises loop with IfIcmple / sum additions in execute_class
        assert_eq!(
            run_class_int("Arithmetic.class", "fibonacci", "(I)I", vec![10]),
            55
        );
    }

    #[test]
    fn ec_arithmetic_sum_to_100() {
        // Exercises += accumulation loop in execute_class
        assert_eq!(
            run_class_int("Arithmetic.class", "sumTo", "(I)I", vec![100]),
            5050
        );
    }

    #[test]
    fn ec_arithmetic_factorial_5() {
        assert_eq!(
            run_class_int("Arithmetic.class", "factorial", "(I)I", vec![5]),
            120
        );
    }

    // =========================================================================
    // Fifth batch: execute() opcode mutant kills
    // =========================================================================

    // ---- execute(): Ldiv / Lrem zero check (== → !=) ----

    #[test]
    fn execute_ldiv_by_zero_errors() {
        // Mutant: replace == with != → division proceeds instead of erroring
        let instructions = vec![
            (0, Instruction::Lconst1),
            (1, Instruction::Lconst0),
            (2, Instruction::Ldiv),
            (3, Instruction::Lreturn),
        ];
        let err = execute(&instructions, &[], vec![], 4, 1).unwrap_err();
        assert!(matches!(err, VmError::DivisionByZero));
    }

    #[test]
    fn execute_lrem_by_zero_errors() {
        let instructions = vec![
            (0, Instruction::Lconst1),
            (1, Instruction::Lconst0),
            (2, Instruction::Lrem),
            (3, Instruction::Lreturn),
        ];
        let err = execute(&instructions, &[], vec![], 4, 1).unwrap_err();
        assert!(matches!(err, VmError::DivisionByZero));
    }

    // ---- execute(): Lor |→^ ----

    #[test]
    fn execute_lor_overlapping_bits_is_not_xor() {
        // a=0b11=3, b=0b10=2 → a|b=3, a^b=1; asserts OR result
        let instructions = vec![
            (0, Instruction::Lconst1), // push 1L
            (1, Instruction::Lconst1), // push 1L
            (2, Instruction::Ladd),    // 1+1=2L
            (3, Instruction::Lconst1), // push 1L
            (4, Instruction::Ladd),    // 2+1=3L  (this is a=3)
            (5, Instruction::Lconst1), // push 1L
            (6, Instruction::Lconst1), // push 1L
            (7, Instruction::Ladd),    // 1+1=2L  (this is b=2)
            (8, Instruction::Lor),     // 3|2=3, but 3^2=1
            (9, Instruction::Lreturn),
        ];
        let r = execute(&instructions, &[], vec![], 8, 1).unwrap().unwrap();
        assert_eq!(r, Slot::Long(3));
    }

    // ---- execute(): Lcmp delete - ----

    #[test]
    fn execute_lcmp_less_than_returns_minus_one() {
        // Mutant: delete - → returns 1 instead of -1
        let instructions = vec![
            (0, Instruction::Lconst0), // push 0L (a)
            (1, Instruction::Lconst1), // push 1L (b)
            (2, Instruction::Lcmp),    // 0 < 1 → -1
            (3, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
        assert_eq!(r, Slot::Int(-1));
    }

    // ---- execute(): IfIcmpeq ==→!= / IfIcmpne !=→== ----

    #[test]
    fn execute_ificmpeq_taken_when_equal() {
        // a==b → jump to return 1; not taken → return 0
        // Mutant ==→!= causes: a==b → not taken → return 0 (fail)
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Iconst2),
            (2, Instruction::IfIcmpeq(5)), // if a==b jump to 2+5=7
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_ificmpeq_not_taken_when_unequal() {
        let instructions = vec![
            (0, Instruction::Iconst1),
            (1, Instruction::Iconst2),
            (2, Instruction::IfIcmpeq(5)), // 1!=2 → not taken
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_ificmpne_taken_when_unequal() {
        // 1!=2 → taken → jump to PC 7 → Iconst1 → return 1
        // Mutant !=→== would check 1==2 (false) → not taken → Iconst0 → return 0
        let instructions = vec![
            (0, Instruction::Iconst1),
            (1, Instruction::Iconst2),
            (2, Instruction::IfIcmpne(5)), // 1!=2 → taken → 2+5=7
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
        assert_eq!(r, Slot::Int(1)); // taken → Iconst1
    }

    #[test]
    fn execute_ificmpne_jump_taken_when_unequal() {
        // 3!=5 → taken → jump to PC 7 → Iconst1 → 1
        let instructions = vec![
            (0, Instruction::Iconst3),
            (1, Instruction::Iconst5),
            (2, Instruction::IfIcmpne(5)), // 3!=5 → taken → 2+5=7
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
        assert_eq!(r, Slot::Int(1)); // taken branch
    }

    #[test]
    fn execute_ificmpne_not_taken_when_equal() {
        // a==b → !=  is false → not taken
        let instructions = vec![
            (0, Instruction::Iconst4),
            (1, Instruction::Iconst4),
            (2, Instruction::IfIcmpne(5)), // 4!=4 → false → not taken
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    // ---- execute(): Newarray negative count → error ----

    #[test]
    fn execute_newarray_negative_count_errors() {
        // count=-1 → NegativeArraySize
        let instructions = vec![
            (0, Instruction::IconstM1),
            (1, Instruction::Newarray(ArrayType::Int)),
            (3, Instruction::Areturn),
        ];
        let err = execute(&instructions, &[], vec![], 2, 1).unwrap_err();
        assert!(matches!(err, VmError::NegativeArraySize { .. }));
    }

    #[test]
    fn execute_newarray_zero_count_succeeds() {
        // count=0 is valid; mutant < → <= would reject count=0
        let instructions = vec![
            (0, Instruction::Iconst0),
            (1, Instruction::Newarray(ArrayType::Int)),
            (3, Instruction::Pop), // discard array ref
            (4, Instruction::Iconst1),
            (5, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    // ---- execute(): Iaload / Iastore bounds ----

    #[test]
    fn execute_iaload_valid_idx0_returns_value() {
        // Kills < → == (if == then idx=0 would wrongly error)
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Int)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst0),
            (5, Instruction::Bipush(77i8)),
            (6, Instruction::Iastore),
            (7, Instruction::Aload0),
            (8, Instruction::Iconst0),
            (9, Instruction::Iaload),
            (10, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 4, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Int(77));
    }

    #[test]
    fn execute_iaload_valid_idx1_returns_value() {
        // Kills < → > (if > then idx=1 > 0 → true → wrongly errors)
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Int)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst1),
            (5, Instruction::Bipush(55i8)),
            (6, Instruction::Iastore),
            (7, Instruction::Aload0),
            (8, Instruction::Iconst1),
            (9, Instruction::Iaload),
            (10, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 4, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Int(55));
    }

    #[test]
    fn execute_iaload_oob_at_length_errors() {
        // idx=2, len=2: second cond true, first false → || gives error; && gives no error
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Int)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Bipush(2i8)),
            (5, Instruction::Iaload),
            (6, Instruction::Ireturn),
        ];
        let err = execute(&instructions, &[], vec![], 4, 2).unwrap_err();
        assert!(matches!(
            err,
            VmError::ArrayIndexOutOfBounds {
                index: 2,
                length: 2
            }
        ));
    }

    #[test]
    fn execute_iastore_valid_idx0_stores() {
        // Kills < → == for Iastore
        let instructions = vec![
            (0, Instruction::Bipush(3i8)),
            (1, Instruction::Newarray(ArrayType::Int)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst0),
            (5, Instruction::Bipush(99i8)),
            (6, Instruction::Iastore),
            (7, Instruction::Aload0),
            (8, Instruction::Iconst0),
            (9, Instruction::Iaload),
            (10, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 4, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Int(99));
    }

    #[test]
    fn execute_iastore_valid_idx1_stores() {
        // Kills < → > for Iastore
        let instructions = vec![
            (0, Instruction::Bipush(3i8)),
            (1, Instruction::Newarray(ArrayType::Int)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst1),
            (5, Instruction::Bipush(88i8)),
            (6, Instruction::Iastore),
            (7, Instruction::Aload0),
            (8, Instruction::Iconst1),
            (9, Instruction::Iaload),
            (10, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 4, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Int(88));
    }

    #[test]
    fn execute_iastore_oob_at_length_errors() {
        // Kills || → && for Iastore
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Int)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Bipush(2i8)), // idx=2=len
            (5, Instruction::Iconst1),
            (6, Instruction::Iastore),
            (7, Instruction::Ireturn),
        ];
        let err = execute(&instructions, &[], vec![], 4, 2).unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute(): Laload / Lastore bounds ----

    #[test]
    fn execute_laload_valid_idx0_returns_value() {
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Long)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst0),
            (5, Instruction::Lconst1),
            (6, Instruction::Lastore),
            (7, Instruction::Aload0),
            (8, Instruction::Iconst0),
            (9, Instruction::Laload),
            (10, Instruction::Lreturn),
        ];
        let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Long(1));
    }

    #[test]
    fn execute_laload_valid_idx1_returns_value() {
        // Kills < → > for Laload (valid at idx=1)
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Long)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst1),
            (5, Instruction::Lconst1),
            (6, Instruction::Lastore),
            (7, Instruction::Aload0),
            (8, Instruction::Iconst1),
            (9, Instruction::Laload),
            (10, Instruction::Lreturn),
        ];
        let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Long(1));
    }

    #[test]
    fn execute_laload_oob_at_length_errors() {
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Long)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Bipush(2i8)),
            (5, Instruction::Laload),
            (6, Instruction::Lreturn),
        ];
        let err = execute(&instructions, &[], vec![], 6, 2).unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    #[test]
    fn execute_lastore_valid_idx0_stores() {
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Long)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst0),
            (5, Instruction::Lconst1),
            (6, Instruction::Lastore),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_lastore_valid_idx1_stores() {
        // Kills < → > for Lastore
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Long)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst1),
            (5, Instruction::Lconst1),
            (6, Instruction::Lastore),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_lastore_oob_at_length_errors() {
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Long)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Bipush(2i8)),
            (5, Instruction::Lconst0),
            (6, Instruction::Lastore),
            (7, Instruction::Ireturn),
        ];
        let err = execute(&instructions, &[], vec![], 6, 2).unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute(): Faload / Fastore bounds ----

    #[test]
    fn execute_faload_valid_idx0_returns_value() {
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Float)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst0),
            (5, Instruction::Fconst1),
            (6, Instruction::Fastore),
            (7, Instruction::Aload0),
            (8, Instruction::Iconst0),
            (9, Instruction::Faload),
            (10, Instruction::Freturn),
        ];
        let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Float(1.0));
    }

    #[test]
    fn execute_faload_valid_idx1_returns_value() {
        // Kills < → > for Faload
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Float)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst1),
            (5, Instruction::Fconst1),
            (6, Instruction::Fastore),
            (7, Instruction::Aload0),
            (8, Instruction::Iconst1),
            (9, Instruction::Faload),
            (10, Instruction::Freturn),
        ];
        let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Float(1.0));
    }

    #[test]
    fn execute_faload_oob_at_length_errors() {
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Float)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Bipush(2i8)),
            (5, Instruction::Faload),
            (6, Instruction::Freturn),
        ];
        let err = execute(&instructions, &[], vec![], 6, 2).unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    #[test]
    fn execute_fastore_valid_idx0_stores() {
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Float)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst0),
            (5, Instruction::Fconst1),
            (6, Instruction::Fastore),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_fastore_valid_idx1_stores() {
        // Kills < → > for Fastore
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Float)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst1),
            (5, Instruction::Fconst1),
            (6, Instruction::Fastore),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_fastore_oob_at_length_errors() {
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Float)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Bipush(2i8)),
            (5, Instruction::Fconst0),
            (6, Instruction::Fastore),
            (7, Instruction::Ireturn),
        ];
        let err = execute(&instructions, &[], vec![], 6, 2).unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute(): Daload / Dastore bounds ----

    #[test]
    fn execute_daload_valid_idx0_returns_value() {
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Double)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst0),
            (5, Instruction::Dconst1),
            (6, Instruction::Dastore),
            (7, Instruction::Aload0),
            (8, Instruction::Iconst0),
            (9, Instruction::Daload),
            (10, Instruction::Dreturn),
        ];
        let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Double(1.0));
    }

    #[test]
    fn execute_daload_valid_idx1_returns_value() {
        // Kills < → > for Daload
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Double)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst1),
            (5, Instruction::Dconst1),
            (6, Instruction::Dastore),
            (7, Instruction::Aload0),
            (8, Instruction::Iconst1),
            (9, Instruction::Daload),
            (10, Instruction::Dreturn),
        ];
        let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Double(1.0));
    }

    #[test]
    fn execute_daload_oob_at_length_errors() {
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Double)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Bipush(2i8)),
            (5, Instruction::Daload),
            (6, Instruction::Dreturn),
        ];
        let err = execute(&instructions, &[], vec![], 6, 2).unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    #[test]
    fn execute_dastore_valid_idx0_stores() {
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Double)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst0),
            (5, Instruction::Dconst1),
            (6, Instruction::Dastore),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_dastore_valid_idx1_stores() {
        // Kills < → > for Dastore
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Double)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Iconst1),
            (5, Instruction::Dconst1),
            (6, Instruction::Dastore),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 6, 2).unwrap().unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_dastore_oob_at_length_errors() {
        let instructions = vec![
            (0, Instruction::Bipush(2i8)),
            (1, Instruction::Newarray(ArrayType::Double)),
            (2, Instruction::Astore0),
            (3, Instruction::Aload0),
            (4, Instruction::Bipush(2i8)),
            (5, Instruction::Dconst0),
            (6, Instruction::Dastore),
            (7, Instruction::Ireturn),
        ];
        let err = execute(&instructions, &[], vec![], 6, 2).unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute(): Tableswitch - → + (non-zero low) ----

    #[test]
    fn execute_tableswitch_nonzero_low_matches_key() {
        // key=2, low=1, offsets[key-low=1]=8 → jump to PC 2+8=10 → Bipush(99)
        // Mutant - → +: offsets[key+low=3] → OOB panic (offsets.len()=3)
        let instructions = vec![
            (0, Instruction::Iconst2),
            (
                2,
                Instruction::Tableswitch {
                    default: 6i32, // 2+6=8 → Iconst0
                    low: 1i32,
                    high: 3i32,
                    offsets: vec![6i32, 8i32, 6i32], // key=1→8, key=2→10, key=3→8
                },
            ),
            (8, Instruction::Iconst0),
            (9, Instruction::Ireturn),
            (10, Instruction::Bipush(99i8)),
            (12, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &[], vec![], 4, 1).unwrap().unwrap();
        assert_eq!(r, Slot::Int(99));
    }

    // ---- execute(): Checkcast null match arm deletion ----

    #[test]
    fn execute_checkcast_null_passes() {
        // Mutant: delete null arm → null falls to _ → TypeMismatch error
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("foo".to_string())),
        ];
        let instructions = vec![
            (0, Instruction::AconstNull),
            (1, Instruction::Checkcast(CpIndex(1))),
            (4, Instruction::Iconst1),
            (5, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &cp, vec![], 4, 1).unwrap().unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_checkcast_matching_ref_passes() {
        // Kills == → != (matching class → pass; mutant rejects when class matches)
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("[I".to_string())), // int array class
        ];
        // Create an int array (class_name="[I"), then checkcast to "[I" → should pass
        let instructions = vec![
            (0, Instruction::Bipush(1i8)),
            (1, Instruction::Newarray(ArrayType::Int)),
            (2, Instruction::Checkcast(CpIndex(1))),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &cp, vec![], 4, 1).unwrap().unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    // ---- execute(): Instanceof null match arm deletion ----

    #[test]
    fn execute_instanceof_null_returns_zero() {
        // Mutant: delete null arm → null falls to _ → TypeMismatch error
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("foo".to_string())),
        ];
        let instructions = vec![
            (0, Instruction::AconstNull),
            (1, Instruction::Instanceof(CpIndex(1))),
            (4, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &cp, vec![], 4, 1).unwrap().unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_instanceof_matching_ref_returns_one() {
        // Kills == → != (matching class → 1; mutant returns 0 when class matches)
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("[I".to_string())),
        ];
        let instructions = vec![
            (0, Instruction::Bipush(1i8)),
            (1, Instruction::Newarray(ArrayType::Int)),
            (2, Instruction::Instanceof(CpIndex(1))),
            (5, Instruction::Ireturn),
        ];
        let r = execute(&instructions, &cp, vec![], 4, 1).unwrap().unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    // =========================================================================
    // Fifth batch: native function boundary mutant kills
    // =========================================================================

    // ---- native_string_substring: begin == len should return empty (> → >=) ----

    #[test]
    fn native_substring_begin_equals_len_returns_empty() {
        // "hello".substring(5) → "" (begin=5=len → valid with >, invalid with >=)
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("hello".to_string());
        let mut out: Vec<u8> = Vec::new();
        let r = native_string_substring(
            &[Slot::Reference(Some(this)), Slot::Int(5)],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        let sub_ref = r.as_reference().unwrap();
        assert_eq!(heap.get(sub_ref).unwrap().string_value.as_deref(), Some(""));
    }

    // ---- native_string_substring_range: begin==end returns empty (first > → ==, >= ) ----

    #[test]
    fn native_substring_range_begin_equals_end_returns_empty() {
        // "hello".substring(2, 2) → "" (begin==end → valid with >, invalid with == or >=)
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("hello".to_string());
        let mut out: Vec<u8> = Vec::new();
        let r = native_string_substring_range(
            &[Slot::Reference(Some(this)), Slot::Int(2), Slot::Int(2)],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        let sub_ref = r.as_reference().unwrap();
        assert_eq!(heap.get(sub_ref).unwrap().string_value.as_deref(), Some(""));
    }

    #[test]
    fn native_substring_range_end_equals_len_is_valid() {
        // "hello".substring(0, 5) → "hello" (end=5=len → valid with >, invalid with >=)
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("hello".to_string());
        let mut out: Vec<u8> = Vec::new();
        let r = native_string_substring_range(
            &[Slot::Reference(Some(this)), Slot::Int(0), Slot::Int(5)],
            &mut heap,
            &mut out,
        )
        .unwrap()
        .unwrap();
        let sub_ref = r.as_reference().unwrap();
        assert_eq!(
            heap.get(sub_ref).unwrap().string_value.as_deref(),
            Some("hello")
        );
    }

    #[test]
    fn native_substring_range_begin_greater_than_end_errors() {
        // begin=3 > end=1 → error (first > condition: 3 > 1 → true → error)
        let mut heap = duke_gc::Heap::new();
        let this = heap.allocate_string("hello".to_string());
        let mut out: Vec<u8> = Vec::new();
        let err = native_string_substring_range(
            &[Slot::Reference(Some(this)), Slot::Int(3), Slot::Int(1)],
            &mut heap,
            &mut out,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- format_arg: Long with %X (uppercase) ----

    #[test]
    fn format_arg_long_uppercase_x_spec() {
        // Mutant: delete Long arm → Long falls to _ → returns "0" instead of "FF"
        let mut heap = duke_gc::Heap::new();
        let r = heap.allocate("java/lang/Long".to_string(), 1);
        heap.get_mut(r).unwrap().fields[0] = Slot::Long(255_i64);
        let result = format_arg('X', None, &Slot::Reference(Some(r)), &heap).unwrap();
        assert_eq!(result, "FF");
    }

    // =========================================================================
    // Sixth batch: execute() Ldiv/Lrem + Anewarray + all remaining array types
    //              execute_class() Faload/Fastore + byte/char/short/ref arrays
    //              Tableswitch, Checkcast null, Multianewarray, parse_arg_count
    //              execute_string_concat_recipe + native_string_format
    // =========================================================================

    // ---- execute(): Ldiv/Lrem success (lines 4129, 4137: == → !=) ----

    #[test]
    fn execute_ldiv_returns_quotient() {
        // b=1 ≠ 0; correct: 1==0=false → proceed; mutant !=: 1!=0=true → DivisionByZero
        let r = execute(
            &[
                (0, Instruction::Lconst1),
                (1, Instruction::Lconst1),
                (2, Instruction::Ldiv),
                (3, Instruction::Lreturn),
            ],
            &[None],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(1));
    }

    #[test]
    fn execute_lrem_returns_remainder() {
        // b=1 ≠ 0; correct: 1==0=false → proceed; mutant !=: 1!=0=true → DivisionByZero
        let r = execute(
            &[
                (0, Instruction::Lconst1),
                (1, Instruction::Lconst1),
                (2, Instruction::Lrem),
                (3, Instruction::Lreturn),
            ],
            &[None],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Long(0));
    }

    // ---- execute(): Anewarray count check (line 4498: < → ==, < → >, < → <=) ----

    #[test]
    fn execute_anewarray_zero_count_succeeds() {
        // count=0: kills < → == (0==0→error) and < → <= (0<=0→error)
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("java/lang/Object".to_string())),
        ];
        let r = execute(
            &[
                (0, Instruction::Iconst0),
                (1, Instruction::Anewarray(CpIndex(1))),
                (5, Instruction::Pop),
                (6, Instruction::Iconst1),
                (7, Instruction::Ireturn),
            ],
            &cp,
            vec![],
            4,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_anewarray_one_count_succeeds() {
        // count=1: kills < → > (1>0=true→error; correct: 1<0=false→ok)
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("java/lang/Object".to_string())),
        ];
        let r = execute(
            &[
                (0, Instruction::Iconst1),
                (1, Instruction::Anewarray(CpIndex(1))),
                (5, Instruction::Pop),
                (6, Instruction::Iconst1),
                (7, Instruction::Ireturn),
            ],
            &cp,
            vec![],
            4,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_anewarray_negative_count_errors() {
        // count=-1: correct -1<0=true→error; mutant > 0: -1>0=false→no error
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("java/lang/Object".to_string())),
        ];
        let err = execute(
            &[
                (0, Instruction::IconstM1),
                (1, Instruction::Anewarray(CpIndex(1))),
                (5, Instruction::Pop),
                (6, Instruction::Iconst0),
                (7, Instruction::Ireturn),
            ],
            &cp,
            vec![],
            4,
            1,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::NegativeArraySize { .. }));
    }

    // ---- execute(): Aaload bounds (line 4663: || → &&, < → ==, < → >, < → <=) ----

    #[test]
    fn execute_aaload_valid_idx0_returns_null() {
        // idx=0: kills < → == (0==0→error) and < → <= (0<=0→error)
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("java/lang/Object".to_string())),
        ];
        let r = execute(
            &[
                (0, Instruction::Iconst2),
                (1, Instruction::Anewarray(CpIndex(1))),
                (5, Instruction::Iconst0),
                (6, Instruction::Aaload),
                (7, Instruction::Pop),
                (8, Instruction::Iconst1),
                (9, Instruction::Ireturn),
            ],
            &cp,
            vec![],
            4,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_aaload_valid_idx1_returns_null() {
        // idx=1: kills < → > (1>0=true→error; correct: 1<0=false→ok)
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("java/lang/Object".to_string())),
        ];
        let r = execute(
            &[
                (0, Instruction::Iconst2),
                (1, Instruction::Anewarray(CpIndex(1))),
                (5, Instruction::Iconst1),
                (6, Instruction::Aaload),
                (7, Instruction::Pop),
                (8, Instruction::Iconst1),
                (9, Instruction::Ireturn),
            ],
            &cp,
            vec![],
            4,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_aaload_oob_at_length_errors() {
        // idx=2=length: kills || → && (false && true = false → no error, but panics)
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("java/lang/Object".to_string())),
        ];
        let err = execute(
            &[
                (0, Instruction::Iconst2),
                (1, Instruction::Anewarray(CpIndex(1))),
                (5, Instruction::Iconst2),
                (6, Instruction::Aaload),
                (7, Instruction::Pop),
                (8, Instruction::Iconst0),
                (9, Instruction::Ireturn),
            ],
            &cp,
            vec![],
            4,
            1,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute(): Aastore bounds (line 4680: || → &&, < → ==, < → >, < → <=) ----

    #[test]
    fn execute_aastore_valid_idx0_stores() {
        // idx=0: kills < → == and < → <=; stack: ref, idx=0, null
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("java/lang/Object".to_string())),
        ];
        let r = execute(
            &[
                (0, Instruction::Iconst2),
                (1, Instruction::Anewarray(CpIndex(1))),
                (5, Instruction::Dup),
                (6, Instruction::Iconst0),
                (7, Instruction::AconstNull),
                (8, Instruction::Aastore),
                (9, Instruction::Pop),
                (10, Instruction::Iconst1),
                (11, Instruction::Ireturn),
            ],
            &cp,
            vec![],
            6,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_aastore_valid_idx1_stores() {
        // idx=1: kills < → > (1>0=true→error)
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("java/lang/Object".to_string())),
        ];
        let r = execute(
            &[
                (0, Instruction::Iconst2),
                (1, Instruction::Anewarray(CpIndex(1))),
                (5, Instruction::Dup),
                (6, Instruction::Iconst1),
                (7, Instruction::AconstNull),
                (8, Instruction::Aastore),
                (9, Instruction::Pop),
                (10, Instruction::Iconst1),
                (11, Instruction::Ireturn),
            ],
            &cp,
            vec![],
            6,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_aastore_oob_at_length_errors() {
        // idx=2=length: kills || → &&
        use duke_classfile::types::CpIndex;
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("java/lang/Object".to_string())),
        ];
        let err = execute(
            &[
                (0, Instruction::Iconst2),
                (1, Instruction::Anewarray(CpIndex(1))),
                (5, Instruction::Iconst2),
                (6, Instruction::AconstNull),
                (7, Instruction::Aastore),
                (8, Instruction::Iconst0),
                (9, Instruction::Ireturn),
            ],
            &cp,
            vec![],
            6,
            1,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute(): Baload bounds (line 4697: || → &&, < → ==, < → >, < → <=) ----

    #[test]
    fn execute_baload_valid_idx0_returns_zero() {
        // idx=0: kills < → == (0==0→error) and < → <= (0<=0→error)
        let r = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Byte)),
                (4, Instruction::Iconst0),
                (5, Instruction::Baload),
                (6, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_baload_valid_idx1_returns_zero() {
        // idx=1: kills < → > (1>0=true→error; correct: 1<0=false→ok)
        let r = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Byte)),
                (4, Instruction::Iconst1),
                (5, Instruction::Baload),
                (6, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_baload_oob_at_length_errors() {
        // idx=2=length: kills || → &&
        let err = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Byte)),
                (4, Instruction::Iconst2),
                (5, Instruction::Baload),
                (6, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            4,
            0,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute(): Bastore bounds (line 4714: || → &&, < → ==, < → >, < → <=) ----

    #[test]
    fn execute_bastore_valid_idx0_stores() {
        // idx=0: kills < → == and < → <=; stack: ref, ref, idx=0, val=42
        let r = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Byte)),
                (4, Instruction::Dup),
                (5, Instruction::Iconst0),
                (6, Instruction::Bipush(42i8)),
                (8, Instruction::Bastore),
                (9, Instruction::Pop),
                (10, Instruction::Iconst1),
                (11, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            6,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_bastore_valid_idx1_stores() {
        // idx=1: kills < → > (1>0=true→error)
        let r = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Byte)),
                (4, Instruction::Dup),
                (5, Instruction::Iconst1),
                (6, Instruction::Bipush(55i8)),
                (8, Instruction::Bastore),
                (9, Instruction::Pop),
                (10, Instruction::Iconst1),
                (11, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            6,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_bastore_oob_at_length_errors() {
        // idx=2=length: kills || → &&; stack: ref, idx=2, val=0
        let err = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Byte)),
                (4, Instruction::Iconst2),
                (5, Instruction::Bipush(0i8)),
                (7, Instruction::Bastore),
                (8, Instruction::Iconst0),
                (9, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            6,
            1,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute(): Caload bounds (line 4731: || → &&, < → ==, < → >, < → <=) ----

    #[test]
    fn execute_caload_valid_idx0_returns_zero() {
        // idx=0: kills < → == and < → <=
        let r = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Char)),
                (4, Instruction::Iconst0),
                (5, Instruction::Caload),
                (6, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_caload_valid_idx1_returns_zero() {
        // idx=1: kills < → > (1>0=true→error)
        let r = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Char)),
                (4, Instruction::Iconst1),
                (5, Instruction::Caload),
                (6, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_caload_oob_at_length_errors() {
        // idx=2=length: kills || → &&
        let err = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Char)),
                (4, Instruction::Iconst2),
                (5, Instruction::Caload),
                (6, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            4,
            0,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute(): Castore bounds (line 4748: || → &&, < → ==, < → >, < → <=) ----

    #[test]
    fn execute_castore_valid_idx0_stores() {
        // idx=0: kills < → == and < → <=
        let r = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Char)),
                (4, Instruction::Dup),
                (5, Instruction::Iconst0),
                (6, Instruction::Bipush(65i8)),
                (8, Instruction::Castore),
                (9, Instruction::Pop),
                (10, Instruction::Iconst1),
                (11, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            6,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_castore_valid_idx1_stores() {
        // idx=1: kills < → > (1>0=true→error)
        let r = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Char)),
                (4, Instruction::Dup),
                (5, Instruction::Iconst1),
                (6, Instruction::Bipush(66i8)),
                (8, Instruction::Castore),
                (9, Instruction::Pop),
                (10, Instruction::Iconst1),
                (11, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            6,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_castore_oob_at_length_errors() {
        // idx=2=length: kills || → &&; stack: ref, idx=2, val=0
        let err = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Char)),
                (4, Instruction::Iconst2),
                (5, Instruction::Iconst0),
                (6, Instruction::Castore),
                (7, Instruction::Iconst0),
                (8, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            6,
            1,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute(): Saload bounds (line 4765: || → &&, < → ==, < → >, < → <=) ----

    #[test]
    fn execute_saload_valid_idx0_returns_zero() {
        // idx=0: kills < → == and < → <=
        let r = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Short)),
                (4, Instruction::Iconst0),
                (5, Instruction::Saload),
                (6, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_saload_valid_idx1_returns_zero() {
        // idx=1: kills < → > (1>0=true→error)
        let r = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Short)),
                (4, Instruction::Iconst1),
                (5, Instruction::Saload),
                (6, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            4,
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn execute_saload_oob_at_length_errors() {
        // idx=2=length: kills || → &&
        let err = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Short)),
                (4, Instruction::Iconst2),
                (5, Instruction::Saload),
                (6, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            4,
            0,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute(): Sastore bounds (line 4782: || → &&, < → ==, < → >, < → <=) ----

    #[test]
    fn execute_sastore_valid_idx0_stores() {
        // idx=0: kills < → == and < → <=
        let r = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Short)),
                (4, Instruction::Dup),
                (5, Instruction::Iconst0),
                (6, Instruction::Bipush(10i8)),
                (8, Instruction::Sastore),
                (9, Instruction::Pop),
                (10, Instruction::Iconst1),
                (11, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            6,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_sastore_valid_idx1_stores() {
        // idx=1: kills < → > (1>0=true→error)
        let r = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Short)),
                (4, Instruction::Dup),
                (5, Instruction::Iconst1),
                (6, Instruction::Bipush(20i8)),
                (8, Instruction::Sastore),
                (9, Instruction::Pop),
                (10, Instruction::Iconst1),
                (11, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            6,
            1,
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn execute_sastore_oob_at_length_errors() {
        // idx=2=length: kills || → &&; stack: ref, idx=2, val=0
        let err = execute(
            &[
                (0, Instruction::Bipush(2i8)),
                (2, Instruction::Newarray(ArrayType::Short)),
                (4, Instruction::Iconst2),
                (5, Instruction::Iconst0),
                (6, Instruction::Sastore),
                (7, Instruction::Iconst0),
                (8, Instruction::Ireturn),
            ],
            &[None],
            vec![],
            6,
            1,
        )
        .unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // =====================================================================
    // Sixth batch: execute_class() array bounds
    // =====================================================================

    // ---- execute_class(): Faload valid idx=1 (line 6881: < → >) ----

    #[test]
    fn ec_faload_valid_idx1_returns_value() {
        // idx=1: kills < → > (1>0=true→error; correct: 1<0=false→ok)
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Float)),
            (2, Instruction::Iconst1),
            (3, Instruction::Faload),
            (4, Instruction::Pop),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 4, 0, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    // ---- execute_class(): Fastore valid idx=0, idx=1 (line 6897: < → ==, < → >, < → <=) ----

    #[test]
    fn ec_fastore_valid_idx0_stores() {
        // idx=0: kills < → == (0==0→error) and < → <= (0<=0→error)
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Float)),
            (2, Instruction::Dup),
            (3, Instruction::Iconst0),
            (4, Instruction::Fconst0),
            (5, Instruction::Fastore),
            (6, Instruction::Pop),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn ec_fastore_valid_idx1_stores() {
        // idx=1: kills < → > (1>0=true→error)
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Float)),
            (2, Instruction::Dup),
            (3, Instruction::Iconst1),
            (4, Instruction::Fconst0),
            (5, Instruction::Fastore),
            (6, Instruction::Pop),
            (7, Instruction::Iconst1),
            (8, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    // ---- execute_class(): Aaload/Aastore OOB (lines 6945, 6961: || → &&) ----

    #[test]
    fn ec_aaload_oob_at_length_errors() {
        // Use int array as stand-in; Aaload clones element without type-checking
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Int)),
            (2, Instruction::Iconst2),
            (3, Instruction::Aaload),
            (4, Instruction::Pop),
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
        ];
        let err = execute_class_synthetic(instructions, vec![], 4, 0, "()I").unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    #[test]
    fn ec_aastore_oob_at_length_errors() {
        // idx=2=length: kills || → &&; stack: ref, idx=2, null
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Int)),
            (2, Instruction::Iconst2),
            (3, Instruction::AconstNull),
            (4, Instruction::Aastore),
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
        ];
        let err = execute_class_synthetic(instructions, vec![], 4, 0, "()I").unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute_class(): Baload bounds (line 6977: || → &&, < → >, < → ==, < → <=) ----

    #[test]
    fn ec_baload_valid_idx0_returns_zero() {
        // idx=0: kills < → == and < → <=
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Byte)),
            (2, Instruction::Iconst0),
            (3, Instruction::Baload),
            (4, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 4, 0, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn ec_baload_valid_idx1_returns_zero() {
        // idx=1: kills < → > (1>0=true→error)
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Byte)),
            (2, Instruction::Iconst1),
            (3, Instruction::Baload),
            (4, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 4, 0, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn ec_baload_oob_at_length_errors() {
        // idx=2=length: kills || → &&
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Byte)),
            (2, Instruction::Iconst2),
            (3, Instruction::Baload),
            (4, Instruction::Ireturn),
        ];
        let err = execute_class_synthetic(instructions, vec![], 4, 0, "()I").unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute_class(): Bastore bounds (line 6993: < → >, || → &&, < → ==, < → <=) ----

    #[test]
    fn ec_bastore_valid_idx0_stores() {
        // idx=0: kills < → == and < → <=
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Byte)),
            (2, Instruction::Dup),
            (3, Instruction::Iconst0),
            (4, Instruction::Bipush(42i8)),
            (6, Instruction::Bastore),
            (7, Instruction::Pop),
            (8, Instruction::Iconst1),
            (9, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn ec_bastore_valid_idx1_stores() {
        // idx=1: kills < → > (1>0=true→error)
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Byte)),
            (2, Instruction::Dup),
            (3, Instruction::Iconst1),
            (4, Instruction::Bipush(55i8)),
            (6, Instruction::Bastore),
            (7, Instruction::Pop),
            (8, Instruction::Iconst1),
            (9, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn ec_bastore_oob_at_length_errors() {
        // idx=2=length: kills || → &&; stack: ref, idx=2, val=0
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Byte)),
            (2, Instruction::Iconst2),
            (3, Instruction::Bipush(0i8)),
            (5, Instruction::Bastore),
            (6, Instruction::Iconst0),
            (7, Instruction::Ireturn),
        ];
        let err = execute_class_synthetic(instructions, vec![], 6, 0, "()I").unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute_class(): Caload OOB (line 7009: || → &&) ----

    #[test]
    fn ec_caload_oob_at_length_errors() {
        // idx=2=length: kills || → &&
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Char)),
            (2, Instruction::Iconst2),
            (3, Instruction::Caload),
            (4, Instruction::Ireturn),
        ];
        let err = execute_class_synthetic(instructions, vec![], 4, 0, "()I").unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute_class(): Castore bounds (line 7025: || → &&, < → ==, < → >, < → <=) ----

    #[test]
    fn ec_castore_valid_idx0_stores() {
        // idx=0: kills < → == and < → <=
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Char)),
            (2, Instruction::Dup),
            (3, Instruction::Iconst0),
            (4, Instruction::Bipush(65i8)),
            (6, Instruction::Castore),
            (7, Instruction::Pop),
            (8, Instruction::Iconst1),
            (9, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn ec_castore_valid_idx1_stores() {
        // idx=1: kills < → > (1>0=true→error)
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Char)),
            (2, Instruction::Dup),
            (3, Instruction::Iconst1),
            (4, Instruction::Bipush(66i8)),
            (6, Instruction::Castore),
            (7, Instruction::Pop),
            (8, Instruction::Iconst1),
            (9, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn ec_castore_oob_at_length_errors() {
        // idx=2=length: kills || → &&; stack: ref, idx=2, val=0
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Char)),
            (2, Instruction::Iconst2),
            (3, Instruction::Iconst0),
            (4, Instruction::Castore),
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
        ];
        let err = execute_class_synthetic(instructions, vec![], 6, 0, "()I").unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute_class(): Saload bounds (line 7041: || → &&, < → ==, < → <=, < → >) ----

    #[test]
    fn ec_saload_valid_idx0_returns_zero() {
        // idx=0: kills < → == and < → <=
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Short)),
            (2, Instruction::Iconst0),
            (3, Instruction::Saload),
            (4, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 4, 0, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn ec_saload_valid_idx1_returns_zero() {
        // idx=1: kills < → > (1>0=true→error)
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Short)),
            (2, Instruction::Iconst1),
            (3, Instruction::Saload),
            (4, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 4, 0, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(0));
    }

    #[test]
    fn ec_saload_oob_at_length_errors() {
        // idx=2=length: kills || → &&
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Short)),
            (2, Instruction::Iconst2),
            (3, Instruction::Saload),
            (4, Instruction::Ireturn),
        ];
        let err = execute_class_synthetic(instructions, vec![], 4, 0, "()I").unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute_class(): Sastore bounds (line 7057: || → &&, < → ==, < → >, < → <=) ----

    #[test]
    fn ec_sastore_valid_idx0_stores() {
        // idx=0: kills < → == and < → <=
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Short)),
            (2, Instruction::Dup),
            (3, Instruction::Iconst0),
            (4, Instruction::Bipush(10i8)),
            (6, Instruction::Sastore),
            (7, Instruction::Pop),
            (8, Instruction::Iconst1),
            (9, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn ec_sastore_valid_idx1_stores() {
        // idx=1: kills < → > (1>0=true→error)
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Short)),
            (2, Instruction::Dup),
            (3, Instruction::Iconst1),
            (4, Instruction::Bipush(20i8)),
            (6, Instruction::Sastore),
            (7, Instruction::Pop),
            (8, Instruction::Iconst1),
            (9, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 6, 0, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    #[test]
    fn ec_sastore_oob_at_length_errors() {
        // idx=2=length: kills || → &&; stack: ref, idx=2, val=0
        let instructions = vec![
            (0, Instruction::Iconst2),
            (1, Instruction::Newarray(ArrayType::Short)),
            (2, Instruction::Iconst2),
            (3, Instruction::Iconst0),
            (4, Instruction::Sastore),
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
        ];
        let err = execute_class_synthetic(instructions, vec![], 6, 0, "()I").unwrap_err();
        assert!(matches!(err, VmError::ArrayIndexOutOfBounds { .. }));
    }

    // ---- execute_class(): Tableswitch nonzero low (line 7076: - → +) ----

    #[test]
    fn ec_tableswitch_nonzero_low_matches_key() {
        // key=2, low=1, high=3: offsets[key-low=1]=9 → target_pc=1+9=10 → Bipush(99)
        // Mutant - → +: offsets[(2+1)=3] → index 3 OOB panic
        let instructions = vec![
            (0, Instruction::Iconst2),
            (
                1,
                Instruction::Tableswitch {
                    default: 11i32, // 1+11=12 → default path
                    low: 1i32,
                    high: 3i32,
                    offsets: vec![11i32, 9i32, 11i32], // key=1→12, key=2→10, key=3→12
                },
            ),
            (10, Instruction::Bipush(99i8)),
            (11, Instruction::Ireturn),
            (12, Instruction::Iconst0),
            (13, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 4, 0, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(99));
    }

    // ---- execute_class(): Checkcast null arm deletion (line 7095) ----

    #[test]
    fn ec_checkcast_null_passes() {
        // Kills: delete Slot::Reference(None) arm → null falls to _ → TypeMismatch
        // Null arm exits before CP lookup, so CpIndex(0) (None entry) is safe here
        use duke_classfile::types::CpIndex;
        let instructions = vec![
            (0, Instruction::AconstNull),
            (1, Instruction::Checkcast(CpIndex(0))),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ];
        let r = execute_class_synthetic(instructions, vec![], 4, 1, "()I")
            .unwrap()
            .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    // ---- execute_class(): Multianewarray negative dim (line 7863: < → ==, < → <=) ----

    #[test]
    fn ec_multianewarray_negative_dim_errors() {
        // dim=-1: kills < → == (-1==0=false→no error; correct: -1<0=true→error)
        use duke_classfile::types::CpIndex;
        use std::sync::Arc;
        let instructions = vec![
            (0, Instruction::IconstM1),
            (
                1,
                Instruction::Multianewarray {
                    index: CpIndex(1),
                    dimensions: 1,
                },
            ),
            (4, Instruction::Pop),
            (5, Instruction::Iconst0),
            (6, Instruction::Ireturn),
        ];
        let pc_to_idx: std::collections::HashMap<usize, usize> = instructions
            .iter()
            .enumerate()
            .map(|(i, (pc, _))| (*pc, i))
            .collect();
        let method = MethodEntry {
            name: "syntest".to_string(),
            descriptor: "()I".to_string(),
            instructions: instructions.into(),
            max_stack: 4,
            max_locals: 1,
            exception_table: vec![],
            pc_to_idx: Arc::new(pc_to_idx),
        };
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("[[I".to_string())),
        ];
        let ctx = ClassContext {
            class_name: "SynTest".to_string(),
            super_class: None,
            interfaces: vec![],
            constant_pool: cp,
            methods: vec![method],
            fields: vec![],
            static_fields: vec![],
            instance_field_count: 0,
            bootstrap_methods: vec![],
        };
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = make_simple_loader();
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        let err = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "SynTest",
            "syntest",
            "()I",
            &[],
        )
        .unwrap_err();
        assert!(matches!(err, VmError::NegativeArraySize { .. }));
    }

    #[test]
    fn ec_multianewarray_zero_dim_succeeds() {
        // dim=0: kills < → <= (0<=0=true→error; correct: 0<0=false→ok)
        use duke_classfile::types::CpIndex;
        use std::sync::Arc;
        let instructions = vec![
            (0, Instruction::Iconst0),
            (
                1,
                Instruction::Multianewarray {
                    index: CpIndex(1),
                    dimensions: 1,
                },
            ),
            (4, Instruction::Pop),
            (5, Instruction::Iconst1),
            (6, Instruction::Ireturn),
        ];
        let pc_to_idx: std::collections::HashMap<usize, usize> = instructions
            .iter()
            .enumerate()
            .map(|(i, (pc, _))| (*pc, i))
            .collect();
        let method = MethodEntry {
            name: "syntest".to_string(),
            descriptor: "()I".to_string(),
            instructions: instructions.into(),
            max_stack: 4,
            max_locals: 1,
            exception_table: vec![],
            pc_to_idx: Arc::new(pc_to_idx),
        };
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("[[I".to_string())),
        ];
        let ctx = ClassContext {
            class_name: "SynTest".to_string(),
            super_class: None,
            interfaces: vec![],
            constant_pool: cp,
            methods: vec![method],
            fields: vec![],
            static_fields: vec![],
            instance_field_count: 0,
            bootstrap_methods: vec![],
        };
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = make_simple_loader();
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        let r = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "SynTest",
            "syntest",
            "()I",
            &[],
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    // ---- parse_arg_count/parse_arg_types fuzzing ----

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn fuzz_parse_arg_count(ref s in "\\PC*") {
            let _ = parse_arg_count(s);
        }

        #[test]
        fn fuzz_parse_arg_types(ref s in "\\PC*") {
            let _ = parse_arg_types(s);
        }
    }

    // ---- parse_arg_count: array arm deletion (line 8295) ----

    #[test]
    fn parse_arg_count_primitive_array_param() {
        // ([I)V has 1 param; mutant deletes '[' arm → counts 0
        assert_eq!(parse_arg_count("([I)V"), 1);
    }

    #[test]
    fn parse_arg_count_object_array_param() {
        // ([Ljava/lang/String;)V has 1 param; mutant: falls to _, count stays 0
        assert_eq!(parse_arg_count("([Ljava/lang/String;)V"), 1);
    }

    #[test]
    fn parse_arg_count_two_array_params() {
        // ([I[J)V has 2 params; mutant: 0
        assert_eq!(parse_arg_count("([I[J)V"), 2);
    }

    // ---- execute_string_concat_recipe (lines 3641, 3648, 3650) ----

    #[test]
    fn string_concat_recipe_extra_dynamic_placeholder_silently_skipped() {
        // Recipe "\u{1}\u{1}" with 1 arg: second \u{1} → dyn_idx=1 < len=1 is false → skip
        // Mutant < → <=: 1 <= 1 → true → dynamic_args[1] OOB panic
        let mut heap = duke_gc::Heap::new();
        let r =
            execute_string_concat_recipe("\u{1}\u{1}", &[Slot::Int(42)], &['I'], &[], &mut heap)
                .unwrap();
        let s = heap
            .get(r.as_reference().unwrap())
            .unwrap()
            .string_value
            .as_deref()
            .unwrap();
        assert_eq!(s, "42");
    }

    #[test]
    fn string_concat_recipe_extra_constant_placeholder_silently_skipped() {
        // Recipe "\u{2}\u{2}" with 1 constant: second \u{2} → const_idx=1 < len=1 is false → skip
        // Mutant < → <=: 1 <= 1 → true → constants[1] OOB panic
        let mut heap = duke_gc::Heap::new();
        let r =
            execute_string_concat_recipe("\u{2}\u{2}", &[], &[], &["hello".to_string()], &mut heap)
                .unwrap();
        let s = heap
            .get(r.as_reference().unwrap())
            .unwrap()
            .string_value
            .as_deref()
            .unwrap();
        assert_eq!(s, "hello");
    }

    #[test]
    fn string_concat_recipe_two_constants_both_appended() {
        // Recipe "\u{2}\u{2}" with constants ["A","B"] → "AB"
        // Mutant += → *=: const_idx stays 0 → "AA"
        let mut heap = duke_gc::Heap::new();
        let r = execute_string_concat_recipe(
            "\u{2}\u{2}",
            &[],
            &[],
            &["A".to_string(), "B".to_string()],
            &mut heap,
        )
        .unwrap();
        let s = heap
            .get(r.as_reference().unwrap())
            .unwrap()
            .string_value
            .as_deref()
            .unwrap();
        assert_eq!(s, "AB");
    }

    // ---- native_string_format (lines 2877, 2891, 2904) ----

    #[test]
    fn native_string_format_width_digit_consumed() {
        // "%1s" with arg "hi": width digit '1' consumed → spec='s' → "hi"
        // Mutant < → == at width loop: loop never runs → spec='1' → wrong output
        let mut heap = duke_gc::Heap::new();
        let fmt = heap.allocate_string("%1s".to_string());
        let arr = heap.allocate("[Ljava/lang/Object;".to_string(), 1);
        let elem = heap.allocate_string("hi".to_string());
        heap.get_mut(arr).unwrap().fields[0] = Slot::Reference(Some(elem));
        let mut sink: Vec<u8> = Vec::new();
        let r = native_string_format(
            &[Slot::Reference(Some(fmt)), Slot::Reference(Some(arr))],
            &mut heap,
            &mut sink,
        )
        .unwrap()
        .unwrap();
        let result = heap
            .get(r.as_reference().unwrap())
            .unwrap()
            .string_value
            .clone()
            .unwrap();
        assert_eq!(result, "hi");
    }

    #[test]
    fn native_string_format_extra_spec_gets_null_arg() {
        // "%s%s" with 1 arg "A": second %s → null (arg_idx=1 not < arr_len=1)
        // Mutant < → <=: arg_idx=1 <= 1 → true → fields[1] OOB panic
        let mut heap = duke_gc::Heap::new();
        let fmt = heap.allocate_string("%s%s".to_string());
        let arr = heap.allocate("[Ljava/lang/Object;".to_string(), 1);
        let a = heap.allocate_string("A".to_string());
        heap.get_mut(arr).unwrap().fields[0] = Slot::Reference(Some(a));
        let mut sink: Vec<u8> = Vec::new();
        let r = native_string_format(
            &[Slot::Reference(Some(fmt)), Slot::Reference(Some(arr))],
            &mut heap,
            &mut sink,
        )
        .unwrap()
        .unwrap();
        let result = heap
            .get(r.as_reference().unwrap())
            .unwrap()
            .string_value
            .clone()
            .unwrap();
        assert_eq!(result, "Anull");
    }

    #[test]
    fn native_string_format_arg_idx_advances_between_specs() {
        // "%s%s" with args ["X","Y"] → "XY"; mutant += → *=: arg_idx stays 0 → "XX"
        let mut heap = duke_gc::Heap::new();
        let fmt = heap.allocate_string("%s%s".to_string());
        let arr = heap.allocate("[Ljava/lang/Object;".to_string(), 2);
        let x = heap.allocate_string("X".to_string());
        let y = heap.allocate_string("Y".to_string());
        heap.get_mut(arr).unwrap().fields[0] = Slot::Reference(Some(x));
        heap.get_mut(arr).unwrap().fields[1] = Slot::Reference(Some(y));
        let mut sink: Vec<u8> = Vec::new();
        let r = native_string_format(
            &[Slot::Reference(Some(fmt)), Slot::Reference(Some(arr))],
            &mut heap,
            &mut sink,
        )
        .unwrap()
        .unwrap();
        let result = heap
            .get(r.as_reference().unwrap())
            .unwrap()
            .string_value
            .clone()
            .unwrap();
        assert_eq!(result, "XY");
    }

    // ---- execute_class: Idiv b==0 check (line 5856 == → !=) ----

    #[test]
    fn ec_idiv_nonzero_denominator_succeeds() {
        // kills == → !=: mutation makes nonzero denominator trigger DivisionByZero
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(10i8)),
                (2, Instruction::Bipush(3i8)),
                (4, Instruction::Idiv),
                (5, Instruction::Ireturn),
            ],
            vec![],
            4,
            0,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(3));
    }

    // ---- execute_class: Ishl bit mask (lines 5876 & → | and & → ^) ----

    #[test]
    fn ec_ishl_masks_shift_count() {
        // a=1, s=1: 1 << (1 & 31 = 1) = 2
        // & → |: 1 << (1 | 31 = 31) = i32::MIN ≠ 2
        // & → ^: 1 << (1 ^ 31 = 30) = 2^30 ≠ 2
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Iconst1),
                (1, Instruction::Iconst1),
                (2, Instruction::Ishl),
                (3, Instruction::Ireturn),
            ],
            vec![],
            4,
            0,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(2));
    }

    // ---- execute_class: Ishr bit mask (lines 5881 & → | and & → ^) ----

    #[test]
    fn ec_ishr_masks_shift_count() {
        // a=-4, s=1: (-4) >> (1 & 31 = 1) = -2
        // & → |: (-4) >> (1 | 31 = 31) = -1 ≠ -2
        // & → ^: (-4) >> (1 ^ 31 = 30) = -1 ≠ -2
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Bipush(-4i8)),
                (2, Instruction::Iconst1),
                (3, Instruction::Ishr),
                (4, Instruction::Ireturn),
            ],
            vec![],
            4,
            0,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(-2));
    }

    // ---- execute_class: Fcmpg a < b (line 6026 < → >) ----

    #[test]
    fn ec_fcmpg_a_less_than_b_returns_minus_one() {
        // 0.0f < 1.0f via Fcmpg: correct = -1
        // < → >: else if a > b fires for NaN arm → returns +1 for Fcmpg
        let r = execute_class_synthetic(
            vec![
                (0, Instruction::Fconst0),
                (1, Instruction::Fconst1),
                (2, Instruction::Fcmpg),
                (3, Instruction::Ireturn),
            ],
            vec![],
            4,
            0,
            "()I",
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(-1));
    }

    // ---- execute_class: LdcW String (line 5624 delete Utf8 arm) ----

    #[test]
    fn ec_ldcw_string_pushes_nonnull_ref() {
        use duke_classfile::types::CpIndex;
        use std::sync::Arc;
        // CP: [0]=None, [1]=String{string_index:2}, [2]=Utf8("hi")
        let cp = vec![
            None,
            Some(CpEntry::String {
                string_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("hi".to_string())),
        ];
        // LdcW → Pop (discards) → Iconst1 → Ireturn
        // With Utf8 arm deletion: LdcW errors → test fails → caught
        let pc_to_idx: std::collections::HashMap<usize, usize> =
            [(0, 0), (3, 1), (4, 2), (5, 3)].iter().copied().collect();
        let method = MethodEntry {
            name: "syntest".to_string(),
            descriptor: "()I".to_string(),
            instructions: std::sync::Arc::from(
                vec![
                    (0, Instruction::LdcW(CpIndex(1))),
                    (3, Instruction::Pop),
                    (4, Instruction::Iconst1),
                    (5, Instruction::Ireturn),
                ]
                .into_boxed_slice(),
            ),
            max_stack: 4,
            max_locals: 0,
            exception_table: vec![],
            pc_to_idx: Arc::new(pc_to_idx),
        };
        let ctx = ClassContext {
            class_name: "SynTest".to_string(),
            super_class: None,
            interfaces: vec![],
            constant_pool: cp,
            methods: vec![method],
            fields: vec![],
            static_fields: vec![],
            instance_field_count: 0,
            bootstrap_methods: vec![],
        };
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = make_simple_loader();
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        let r = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "SynTest",
            "syntest",
            "()I",
            &[],
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    // ---- execute_class: LdcW Class (line 5653 delete Utf8 arm) ----

    #[test]
    fn ec_ldcw_class_constant_pushes_nonnull_ref() {
        use duke_classfile::types::CpIndex;
        use std::sync::Arc;
        // CP: [0]=None, [1]=Class{name_index:2}, [2]=Utf8("java/lang/Object")
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("java/lang/Object".to_string())),
        ];
        // LdcW(Class) → Pop → Iconst1 → Ireturn
        // With Utf8 arm deletion in LdcW Class branch: class_info=None → ldc_push with Class entry → InvalidCpIndex
        let pc_to_idx: std::collections::HashMap<usize, usize> =
            [(0, 0), (3, 1), (4, 2), (5, 3)].iter().copied().collect();
        let method = MethodEntry {
            name: "syntest".to_string(),
            descriptor: "()I".to_string(),
            instructions: std::sync::Arc::from(
                vec![
                    (0, Instruction::LdcW(CpIndex(1))),
                    (3, Instruction::Pop),
                    (4, Instruction::Iconst1),
                    (5, Instruction::Ireturn),
                ]
                .into_boxed_slice(),
            ),
            max_stack: 4,
            max_locals: 0,
            exception_table: vec![],
            pc_to_idx: Arc::new(pc_to_idx),
        };
        let ctx = ClassContext {
            class_name: "SynTest".to_string(),
            super_class: None,
            interfaces: vec![],
            constant_pool: cp,
            methods: vec![method],
            fields: vec![],
            static_fields: vec![],
            instance_field_count: 0,
            bootstrap_methods: vec![],
        };
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let loader = make_simple_loader();
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        let r = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "SynTest",
            "syntest",
            "()I",
            &[],
        )
        .unwrap()
        .unwrap();
        assert_eq!(r, Slot::Int(1));
    }

    // ---- init_object_fields (lines 8497,8513,8516,8518,8522) ----

    #[allow(clippy::too_many_lines)]
    #[test]
    fn ec_new_initialises_reference_field_to_null() {
        // Class "RefBox" with: 1 static int field + 2 instance fields (int then reference).
        // After `new RefBox`, getfield refField should return Reference(None), not Int(0).
        //
        // Kills:
        //   8497 (skip init): refField stays Int(0) → ifnull fails → return 0
        //   8513 (include statics in slot count): static offset shifts, refField goes OOB → not written
        //   8516 (write only Int(0) defaults): refField (Reference type) skipped → stays Int(0)
        //   8518 (< → ==): slot_idx never == len → never writes → stays Int(0)
        //   8518 (< → >): 0 > 2 = false → never writes
        //   8522 (+= → *=1): slot_idx stays 0, writes to intField slot → refField unchanged
        use duke_classfile::types::CpIndex;
        use std::sync::Arc;

        // CP for "SynTest" calling class:
        // [0]=None, [1]=Class("RefBox"), [2]=Utf8("RefBox"),
        // [3]=Fieldref(class=1, nat=4), [4]=NameAndType(name=5,desc=6),
        // [5]=Utf8("refField"), [6]=Utf8("Ljava/lang/Object;")
        let cp = vec![
            None,
            Some(CpEntry::Class {
                name_index: CpIndex(2),
            }),
            Some(CpEntry::Utf8("RefBox".to_string())),
            Some(CpEntry::Fieldref {
                class_index: CpIndex(1),
                name_and_type_index: CpIndex(4),
            }),
            Some(CpEntry::NameAndType {
                name_index: CpIndex(5),
                descriptor_index: CpIndex(6),
            }),
            Some(CpEntry::Utf8("refField".to_string())),
            Some(CpEntry::Utf8("Ljava/lang/Object;".to_string())),
        ];

        // Instructions:
        // 0: new RefBox
        // 3: getfield refField → pushes fields[1] (slot 1 of instance)
        // 6: ifnull 4           → if null: jump to 6+4=10 (success: field is null as expected)
        // 8: iconst0            → not null: fail
        // 9: ireturn
        // 10: iconst1           → success
        // 11: ireturn
        let instructions = vec![
            (0usize, Instruction::New(CpIndex(1))),
            (3, Instruction::Getfield(CpIndex(3))),
            (6, Instruction::Ifnull(4i16)),
            (8, Instruction::Iconst0),
            (9, Instruction::Ireturn),
            (10, Instruction::Iconst1),
            (11, Instruction::Ireturn),
        ];
        let pc_to_idx: std::collections::HashMap<usize, usize> = instructions
            .iter()
            .enumerate()
            .map(|(i, (pc, _))| (*pc, i))
            .collect();
        let method = MethodEntry {
            name: "syntest".to_string(),
            descriptor: "()I".to_string(),
            instructions: instructions.into(),
            max_stack: 4,
            max_locals: 0,
            exception_table: vec![],
            pc_to_idx: Arc::new(pc_to_idx),
        };
        let caller_ctx = ClassContext {
            class_name: "SynTest".to_string(),
            super_class: None,
            interfaces: vec![],
            constant_pool: cp,
            methods: vec![method],
            fields: vec![],
            static_fields: vec![],
            instance_field_count: 0,
            bootstrap_methods: vec![],
        };

        // RefBox: 1 static field "count:I" + 2 instance fields "intField:I", "refField:Ljava/lang/Object;"
        let refbox_ctx = ClassContext {
            class_name: "RefBox".to_string(),
            super_class: None,
            interfaces: vec![],
            constant_pool: vec![None],
            methods: vec![],
            fields: vec![
                FieldEntry {
                    name: "count".to_string(),
                    descriptor: "I".to_string(),
                    is_static: true,
                },
                FieldEntry {
                    name: "intField".to_string(),
                    descriptor: "I".to_string(),
                    is_static: false,
                },
                FieldEntry {
                    name: "refField".to_string(),
                    descriptor: "Ljava/lang/Object;".to_string(),
                    is_static: false,
                },
            ],
            static_fields: vec![Slot::Int(0)],
            instance_field_count: 2,
            bootstrap_methods: vec![],
        };

        let mut registry = ClassRegistry::new();
        registry.register(caller_ctx);
        registry.register(refbox_ctx);
        let loader = make_simple_loader();
        let mut heap = duke_gc::Heap::new();
        let mut sink: Vec<u8> = Vec::new();
        let r = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut sink,
            "SynTest",
            "syntest",
            "()I",
            &[],
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            r,
            Slot::Int(1),
            "refField should be Reference(None) after init"
        );
    }

    // ---- array_list_sort negative size (lines 9074 guard, 9075 arm deletion) ----

    #[test]
    fn array_list_sort_negative_size_returns_error() {
        // Allocate an ArrayList-like object with fields[0] = -1 (negative size).
        // kills 9074 (guard *n >= 0 → true): negative n accepted as huge usize →
        //   elems.len() != size → InvalidRef (not NegativeArraySize)
        // kills 9075 (delete second arm): negative n falls to _ → Ok(None), not error
        let mut heap = duke_gc::Heap::new();
        let list_ref = heap.allocate("java/util/ArrayList".to_string(), 1);
        heap.get_mut(list_ref).unwrap().fields[0] = Slot::Int(-1);
        let mut sink: Vec<u8> = Vec::new();
        let args = [Slot::Reference(Some(list_ref)), Slot::Reference(None)];
        let mut ops = NoopCallbackOps;
        let err = array_list_sort(&args, &mut heap, &mut sink, &mut ops).unwrap_err();
        assert!(
            matches!(err, VmError::NegativeArraySize { .. }),
            "expected NegativeArraySize, got {err:?}"
        );
    }

    // ---- Phase 30: Reflection fixture coverage ----

    #[test]
    fn reflection_for_name_and_get_name() {
        assert_eq!(
            run_bootstrap_int("ReflectionTest.class", "forNameAndGetName", "()I"),
            1
        );
    }

    #[test]
    fn reflection_string_class_literal_uses_binary_name() {
        assert_eq!(
            run_bootstrap_int(
                "ReflectionTest.class",
                "stringClassLiteralUsesBinaryName",
                "()I"
            ),
            1
        );
    }

    #[test]
    fn reflection_declared_methods_include_public_and_private() {
        assert_eq!(
            run_bootstrap_int(
                "ReflectionTest.class",
                "declaredMethodsIncludePublicAndPrivate",
                "()I",
            ),
            5
        );
    }

    #[test]
    fn reflection_declared_fields_include_public_and_private() {
        assert_eq!(
            run_bootstrap_int(
                "ReflectionTest.class",
                "declaredFieldsIncludePublicAndPrivate",
                "()I",
            ),
            2
        );
    }

    #[test]
    fn reflection_invoke_static_and_instance_methods() {
        assert_eq!(
            run_bootstrap_int("ReflectionTest.class", "invokeStaticAdd", "()I"),
            7
        );
        assert_eq!(
            run_bootstrap_int("ReflectionTest.class", "invokeInstanceTimes", "()I"),
            21
        );
    }

    #[test]
    fn reflection_invoke_long_primitive_round_trips() {
        assert_eq!(
            run_bootstrap_int("ReflectionTest.class", "invokeStaticAddLong", "()I"),
            1
        );
    }

    #[test]
    fn reflection_missing_class_raises_class_not_found() {
        assert_eq!(
            run_bootstrap_int(
                "ReflectionTest.class",
                "missingClassRaisesClassNotFound",
                "()I"
            ),
            1
        );
    }

    #[test]
    fn reflection_private_method_invoke_raises_illegal_access() {
        assert_eq!(
            run_bootstrap_int(
                "ReflectionTest.class",
                "privateMethodRaisesIllegalAccess",
                "()I"
            ),
            1
        );
    }

    #[test]
    fn reflection_target_exception_is_wrapped() {
        assert_eq!(
            run_bootstrap_int("ReflectionTest.class", "targetExceptionIsWrapped", "()I"),
            1
        );
    }

    // ---- Phase 32: Process management fixture coverage ----

    fn repo_root_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn host_process_command() -> String {
        if cfg!(windows) {
            "powershell".to_string()
        } else {
            "sh".to_string()
        }
    }

    #[test]
    fn process_spawn_and_read_stdout_returns_expected_sum() {
        let result = run_bootstrap_with_string_args(
            "ProcessManagementTest.class",
            "spawnAndReadStdout",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
            &[
                host_process_command(),
                fixtures_dir().to_string_lossy().into_owned(),
                repo_root_dir().to_string_lossy().into_owned(),
            ],
        );

        assert!(
            result.is_ok(),
            "expected ProcessBuilder.start stdout support; current Duke failed with {result:?}"
        );
        assert_eq!(result.unwrap(), Some(Slot::Int(1)));
    }

    #[test]
    fn process_runtime_exec_reads_stdout_and_exit_code() {
        let result = run_bootstrap_with_string_args(
            "ProcessManagementTest.class",
            "runtimeExecReadsStdout",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
            &[
                host_process_command(),
                fixtures_dir().to_string_lossy().into_owned(),
                repo_root_dir().to_string_lossy().into_owned(),
            ],
        );

        assert!(
            result.is_ok(),
            "expected Runtime.exec stdout support; current Duke failed with {result:?}"
        );
        assert_eq!(result.unwrap(), Some(Slot::Int(1)));
    }

    #[test]
    fn process_builder_applies_working_directory() {
        let result = run_bootstrap_with_string_args(
            "ProcessManagementTest.class",
            "processBuilderAppliesWorkingDirectory",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
            &[
                host_process_command(),
                fixtures_dir().to_string_lossy().into_owned(),
                repo_root_dir().to_string_lossy().into_owned(),
            ],
        );

        assert!(
            result.is_ok(),
            "expected ProcessBuilder.directory working directory support; current Duke failed with {result:?}"
        );
        assert_eq!(result.unwrap(), Some(Slot::Int(1)));
    }

    #[test]
    fn process_runtime_exec_applies_working_directory() {
        let result = run_bootstrap_with_string_args(
            "ProcessManagementTest.class",
            "runtimeExecAppliesWorkingDirectory",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
            &[
                host_process_command(),
                fixtures_dir().to_string_lossy().into_owned(),
                repo_root_dir().to_string_lossy().into_owned(),
            ],
        );

        assert!(
            result.is_ok(),
            "expected Runtime.exec working directory support; current Duke failed with {result:?}"
        );
        assert_eq!(result.unwrap(), Some(Slot::Int(1)));
    }

    #[test]
    fn process_pipe_stdin_to_child_round_trips() {
        let result = run_bootstrap_with_string_args(
            "ProcessManagementTest.class",
            "pipeStdinToChild",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
            &[
                host_process_command(),
                fixtures_dir().to_string_lossy().into_owned(),
                repo_root_dir().to_string_lossy().into_owned(),
            ],
        );

        assert!(
            result.is_ok(),
            "expected child stdin/stdout pipe support; current Duke failed with {result:?}"
        );
        assert_eq!(result.unwrap(), Some(Slot::Int(1)));
    }

    #[test]
    fn process_read_error_stream_returns_expected_sum() {
        let result = run_bootstrap_with_string_args(
            "ProcessManagementTest.class",
            "readErrorStream",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
            &[
                host_process_command(),
                fixtures_dir().to_string_lossy().into_owned(),
                repo_root_dir().to_string_lossy().into_owned(),
            ],
        );

        assert!(
            result.is_ok(),
            "expected child stderr pipe support; current Duke failed with {result:?}"
        );
        assert_eq!(result.unwrap(), Some(Slot::Int(1)));
    }

    #[test]
    fn process_destroy_terminates_sleeping_child() {
        let result = run_bootstrap_with_string_args(
            "ProcessManagementTest.class",
            "destroySleepingChild",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
            &[
                host_process_command(),
                fixtures_dir().to_string_lossy().into_owned(),
                repo_root_dir().to_string_lossy().into_owned(),
            ],
        );

        assert!(
            result.is_ok(),
            "expected Process.destroy and exitValue support; current Duke failed with {result:?}"
        );
        assert_eq!(result.unwrap(), Some(Slot::Int(1)));
    }

    #[test]
    fn process_missing_executable_raises_io_exception() {
        let result = run_bootstrap_int(
            "ProcessManagementTest.class",
            "missingExecutableRaisesIoException",
            "()I",
        );
        assert_eq!(result, 1);
    }

    // ---- Phase 29: Networking helpers and integration tests ----

    fn run_bootstrap_with_slots(
        class_name: &str,
        method_name: &str,
        descriptor: &str,
        args: &[Slot],
    ) -> VmResult<Option<Slot>> {
        let ctx = load_class_context(class_name);
        let entry_class = ctx.class_name.clone();
        let mut registry = ClassRegistry::new();
        registry.register(ctx);
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        let loader = fixtures_loader();
        let mut out: Vec<u8> = Vec::new();
        execute_class_to_completion(
            &mut registry,
            loader,
            &mut heap,
            &mut out,
            &entry_class,
            method_name,
            descriptor,
            args,
        )
    }

    #[test]
    fn net_bind_and_get_port() {
        let port = run_bootstrap_int("NetworkingTest.class", "bindAndGetPort", "()I");
        assert!(port > 0, "expected positive port, got {port}");
    }

    #[test]
    fn net_connect_refused() {
        // Bind then immediately drop so nothing listens on that port
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        // Brief sleep ensures the OS releases the port before Java tries to connect.
        // No TIME_WAIT applies (listener was never accepted-on), but the sleep guards
        // against any OS-specific teardown delay.
        std::thread::sleep(std::time::Duration::from_millis(10));
        let result = run_bootstrap_with_slots(
            "NetworkingTest.class",
            "connectRefused",
            "(I)I",
            &[Slot::Int(i32::from(port))],
        )
        .expect("connectRefused failed");
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn net_accept_and_read() {
        use std::io::Read;
        // Rust is the server — no drop, no rebind race
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1];
            stream.read_exact(&mut buf).unwrap();
            assert_eq!(buf[0], 42);
        });
        // Java is the client, writes byte 42
        let result = run_bootstrap_with_slots(
            "NetworkingTest.class",
            "connectAndWriteByte",
            "(II)I",
            &[Slot::Int(i32::from(port)), Slot::Int(42)],
        )
        .expect("connectAndWriteByte failed");
        handle.join().unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn net_connect_and_write() {
        // Rust side: listen; Java side: connect and write byte 99
        use std::io::Read;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1];
            stream.read_exact(&mut buf).unwrap();
            assert_eq!(buf[0], 99);
        });

        let result = run_bootstrap_with_slots(
            "NetworkingTest.class",
            "connectAndWriteByte",
            "(II)I",
            &[Slot::Int(i32::from(port)), Slot::Int(99)],
        )
        .expect("connectAndWriteByte failed");
        handle.join().unwrap();
        assert_eq!(result, Some(Slot::Int(1)));
    }

    #[test]
    fn net_echo_roundtrip() {
        use std::io::{Read, Write};
        // Rust is the echo server — keeps listener alive, no race
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            // Echo each byte individually so Java's write-then-read loop doesn't deadlock
            let mut buf = [0u8; 1];
            for _ in 0..3 {
                stream.read_exact(&mut buf).unwrap();
                stream.write_all(&buf).unwrap();
            }
        });
        // Java is the client — connects, writes [1,2,3], reads them back
        let result = run_bootstrap_with_slots(
            "NetworkingTest.class",
            "connectAndEchoCheck",
            "(II)I",
            &[Slot::Int(i32::from(port)), Slot::Int(3)],
        )
        .expect("connectAndEchoCheck failed");
        handle.join().unwrap();
        assert_eq!(result, Some(Slot::Int(3)));
    }

    // ── ZIP / JAR tests ─────────────────────────────────────────────────

    fn fixtures_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests")
            .join("fixtures")
    }

    #[test]
    fn zip_loader_loads_class_from_jar() {
        let jar_path = fixtures_dir().join("hello.jar");
        let loader = duke_loader::ZipLoader::open(&jar_path).expect("should open hello.jar");
        let bytes = loader
            .find_class("HelloWorld")
            .expect("should find HelloWorld in JAR");
        assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
    }

    #[test]
    fn execute_class_loaded_from_jar() {
        let jar_path = fixtures_dir().join("hello.jar");
        let loader = duke_loader::ZipLoader::open(&jar_path).expect("open hello.jar");
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        // Pre-load the entry class from the JAR.
        registry
            .ensure_loaded("HelloWorld", &loader)
            .expect("load HelloWorld");
        // Build String[] args
        let arr_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
        let main_args = vec![Slot::Reference(Some(arr_ref))];
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class_to_completion(
            &mut registry,
            loader,
            &mut heap,
            &mut out,
            "HelloWorld",
            "main",
            "([Ljava/lang/String;)V",
            &main_args,
        );
        assert!(
            result.is_ok(),
            "HelloWorld.main from JAR failed: {}",
            result.unwrap_err()
        );
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Hello, World!"), "output was: {output}");
    }

    #[test]
    fn multi_class_jar_loading() {
        let jar_path = fixtures_dir().join("multi.jar");
        let loader = duke_loader::ZipLoader::open(&jar_path).expect("open multi.jar");
        let mut registry = ClassRegistry::new();
        let mut heap = duke_gc::Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);
        // Pre-load the entry class from the JAR.
        registry
            .ensure_loaded("MultiClassJar", &loader)
            .expect("load MultiClassJar");
        let mut out: Vec<u8> = Vec::new();
        let result = execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            "MultiClassJar",
            "compute",
            "()I",
            &[],
        )
        .expect("MultiClassJar.compute should succeed");
        assert_eq!(result, Some(Slot::Int(99)));
    }

    #[test]
    fn zip_file_native_entry_count() {
        let jar_path = fixtures_dir().join("hello.jar");
        let jar_str = jar_path.to_str().unwrap().to_string();
        let result = run_bootstrap_with_string_args(
            "ZipReadTest.class",
            "entryCount",
            "(Ljava/lang/String;)I",
            &[jar_str],
        )
        .expect("entryCount should succeed");
        // hello.jar contains HelloWorld.class + META-INF/MANIFEST.MF
        if let Some(Slot::Int(count)) = result {
            assert!(count >= 2, "expected at least 2 entries, got {count}");
        } else {
            panic!("expected Int result, got {result:?}");
        }
    }

    #[test]
    fn zip_file_native_read_first_byte() {
        // Use hello.jar — read the first byte of HelloWorld.class (0xCA = 202).
        let jar_path = fixtures_dir().join("hello.jar");
        let path_str = jar_path.to_str().unwrap().to_string();
        let result = run_bootstrap_with_string_args(
            "ZipReadTest.class",
            "readFirstByte",
            "(Ljava/lang/String;Ljava/lang/String;)I",
            &[path_str, "HelloWorld.class".to_string()],
        )
        .expect("readFirstByte should succeed");
        // 0xCA = 202 (first byte of .class magic number)
        assert_eq!(result, Some(Slot::Int(0xCA)));
    }
}
#[cfg(test)]
mod fuzz;
