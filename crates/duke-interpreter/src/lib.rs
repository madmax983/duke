//! Switch-dispatch JVM bytecode interpreter for Duke Phase 4.
//!
//! Executes decoded instruction streams for methods containing integer, long,
//! float, and double arithmetic, control flow, and local variables.  Heap
//! allocation, field access, and method invocation are not yet implemented.

/// Core execution context types for methods and classes.
pub mod context;
/// Repositories for loaded classes and registered native methods.
pub mod registry;

pub use context::*;
pub use registry::*;

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;

use duke_bytecode::Instruction;
use duke_bytecode::instruction::ArrayType;
use duke_classfile::types::CpEntry;
use duke_loader::ClassLoader;
use duke_runtime::{Frame, Slot, VmError, VmResult};

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
pub type CallbackNativeHandler = fn(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    output: &mut dyn Write,
    invoke: &mut dyn FnMut(
        &mut duke_gc::Heap,
        &mut dyn Write,
        &str, // class name
        &str, // method name
        &str, // descriptor
        Vec<Slot>,
    ) -> VmResult<Option<Slot>>,
) -> VmResult<Option<Slot>>;

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

    let file_input_stream_ctx = ClassContext {
        class_name: "java/io/FileInputStream".to_string(),
        super_class: Some("java/lang/Object".to_string()),
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
        super_class: Some("java/lang/Object".to_string()),
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
}

fn native_println_string(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
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

fn file_path_from_this(args: &[Slot], heap: &duke_gc::Heap) -> VmResult<std::path::PathBuf> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let path_ref = match heap.get(this_ref)?.fields.first() {
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

fn native_file_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
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
) -> VmResult<Option<Slot>> {
    let path = file_path_from_this(args, heap)?;
    Ok(Some(Slot::Int(i32::from(path.exists()))))
}

fn native_file_is_file(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let path = file_path_from_this(args, heap)?;
    Ok(Some(Slot::Int(i32::from(path.is_file()))))
}

fn native_file_is_directory(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
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
) -> VmResult<Option<Slot>> {
    let file_id = file_stream_id_from_this(args, heap)?;
    Ok(Some(Slot::Int(heap.read_host_file_byte(file_id)?)))
}

fn native_file_input_stream_read_bytes(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
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

/// Native: `String.length()` — returns string length as int.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn native_string_length(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
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
) -> VmResult<Option<Slot>> {
    Ok(None)
}

/// Native: `Enum.<init>(Ljava/lang/String;I)V` — stores name + ordinal.
/// args: `[this_ref, name_ref, ordinal_int]`
fn native_enum_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
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

/// Native: `String.valueOf(int)` — static method, returns string of int.
fn native_string_value_of_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
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
) -> VmResult<Option<Slot>> {
    let code = match args.first() {
        Some(Slot::Int(v)) => *v,
        _ => 1,
    };
    Err(VmError::SystemExit { code })
}

/// Native: `String.substring(int)` — substring from begin to end.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn native_string_substring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
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
) -> VmResult<Option<Slot>> {
    native_string_compareto_object(args, heap, out)
}

/// Native: `String.compareTo(Object)` — lexicographic comparison via Object descriptor.
fn native_string_compareto_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
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
) -> VmResult<Option<Slot>> {
    Ok(Some(args.first().copied().unwrap_or(Slot::Reference(None))))
}

// ---- Math natives ----

/// Native: `Math.max(int, int)` — returns the larger value.
fn native_math_max_int(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
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
fn execute_string_concat_recipe(
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

/// Execute a decoded JVM instruction stream.
///
/// # Parameters
/// - `instructions`: output of `duke_bytecode::decode`
/// - `cp`: constant pool from the parsed `ClassFile`
/// - `args`: initial local variable values (method arguments)
/// - `max_stack`: `Code.max_stack` from the class file
/// - `max_locals`: `Code.max_locals` from the class file
///
/// # Returns
/// `Ok(Some(slot))` for value-returning methods, `Ok(None)` for `void`.
///
/// # Errors
/// Returns [`VmError`] on execution faults (division by zero, stack overflow,
/// unimplemented instruction, etc.).
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
pub fn execute(
    instructions: &[(usize, Instruction)],
    cp: &[Option<CpEntry>],
    args: Vec<Slot>,
    max_stack: u16,
    max_locals: u16,
) -> VmResult<Option<Slot>> {
    // Build PC → instruction-index map for O(1) branch resolution.
    let pc_to_idx: HashMap<usize, usize> = instructions
        .iter()
        .enumerate()
        .map(|(i, &(pc, _))| (pc, i))
        .collect();

    let mut frame = Frame::new(usize::from(max_stack), usize::from(max_locals), args)?;
    let mut idx: usize = 0;
    // Local heap for array objects allocated during single-method execution.
    let mut local_heap: Vec<(String, Vec<Slot>, Option<String>)> = Vec::new();
    let mut string_intern: HashMap<usize, u64> = HashMap::new();

    loop {
        let Some((pc, instr)) = instructions.get(idx) else {
            return Err(VmError::FellOffEnd);
        };
        let pc = *pc;

        // Jump to a PC-relative branch target (offset relative to current `pc`).
        macro_rules! jump {
            ($offset:expr) => {{
                let target = (pc as i64).wrapping_add(i64::from($offset)) as usize;
                idx = *pc_to_idx
                    .get(&target)
                    .ok_or(VmError::InvalidBranchTarget { pc: target })?;
                continue;
            }};
        }

        match instr {
            // ----------------------------------------------------------------
            // Constants
            // ----------------------------------------------------------------
            Instruction::Nop => {}
            Instruction::AconstNull => frame.push(Slot::Reference(None))?,
            Instruction::IconstM1 => frame.push(Slot::Int(-1))?,
            Instruction::Iconst0 => frame.push(Slot::Int(0))?,
            Instruction::Iconst1 => frame.push(Slot::Int(1))?,
            Instruction::Iconst2 => frame.push(Slot::Int(2))?,
            Instruction::Iconst3 => frame.push(Slot::Int(3))?,
            Instruction::Iconst4 => frame.push(Slot::Int(4))?,
            Instruction::Iconst5 => frame.push(Slot::Int(5))?,
            Instruction::Lconst0 => frame.push(Slot::Long(0))?,
            Instruction::Lconst1 => frame.push(Slot::Long(1))?,
            Instruction::Fconst0 => frame.push(Slot::Float(0.0))?,
            Instruction::Fconst1 => frame.push(Slot::Float(1.0))?,
            Instruction::Fconst2 => frame.push(Slot::Float(2.0))?,
            Instruction::Dconst0 => frame.push(Slot::Double(0.0))?,
            Instruction::Dconst1 => frame.push(Slot::Double(1.0))?,
            Instruction::Bipush(v) => frame.push(Slot::Int(i32::from(*v)))?,
            Instruction::Sipush(v) => frame.push(Slot::Int(i32::from(*v)))?,

            // ----------------------------------------------------------------
            // Constant pool load
            // ----------------------------------------------------------------
            Instruction::Ldc(raw_idx) => {
                let cp_idx = usize::from(*raw_idx);
                if let Some(CpEntry::String { string_index }) =
                    cp.get(cp_idx).and_then(|e| e.as_ref())
                {
                    let si = string_index.0 as usize;
                    let r = if let Some(&cached) = string_intern.get(&cp_idx) {
                        cached
                    } else {
                        let s = match cp.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        let r = local_heap.len() as u64;
                        local_heap.push(("java/lang/String".to_string(), Vec::new(), Some(s)));
                        string_intern.insert(cp_idx, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    ldc_push(&mut frame, cp, cp_idx)?;
                }
            }
            Instruction::LdcW(cp_idx) | Instruction::Ldc2W(cp_idx) => {
                let idx_val = usize::from(cp_idx.0);
                if let Some(CpEntry::String { string_index }) =
                    cp.get(idx_val).and_then(|e| e.as_ref())
                {
                    let si = string_index.0 as usize;
                    let r = if let Some(&cached) = string_intern.get(&idx_val) {
                        cached
                    } else {
                        let s = match cp.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        let r = local_heap.len() as u64;
                        local_heap.push(("java/lang/String".to_string(), Vec::new(), Some(s)));
                        string_intern.insert(idx_val, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    ldc_push(&mut frame, cp, idx_val)?;
                }
            }

            // ----------------------------------------------------------------
            // Loads
            // ----------------------------------------------------------------
            Instruction::Iload(i)
            | Instruction::Lload(i)
            | Instruction::Fload(i)
            | Instruction::Dload(i)
            | Instruction::Aload(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }
            Instruction::Iload0
            | Instruction::Lload0
            | Instruction::Fload0
            | Instruction::Dload0
            | Instruction::Aload0 => {
                let s = frame.load_local(0)?;
                frame.push(s)?;
            }
            Instruction::Iload1
            | Instruction::Lload1
            | Instruction::Fload1
            | Instruction::Dload1
            | Instruction::Aload1 => {
                let s = frame.load_local(1)?;
                frame.push(s)?;
            }
            Instruction::Iload2
            | Instruction::Lload2
            | Instruction::Fload2
            | Instruction::Dload2
            | Instruction::Aload2 => {
                let s = frame.load_local(2)?;
                frame.push(s)?;
            }
            Instruction::Iload3
            | Instruction::Lload3
            | Instruction::Fload3
            | Instruction::Dload3
            | Instruction::Aload3 => {
                let s = frame.load_local(3)?;
                frame.push(s)?;
            }
            Instruction::IloadW(i)
            | Instruction::LloadW(i)
            | Instruction::FloadW(i)
            | Instruction::DloadW(i)
            | Instruction::AloadW(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }

            // ----------------------------------------------------------------
            // Stores
            // ----------------------------------------------------------------
            Instruction::Istore(i)
            | Instruction::Lstore(i)
            | Instruction::Fstore(i)
            | Instruction::Dstore(i)
            | Instruction::Astore(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }
            Instruction::Istore0
            | Instruction::Lstore0
            | Instruction::Fstore0
            | Instruction::Dstore0
            | Instruction::Astore0 => {
                let v = frame.pop()?;
                frame.store_local(0, v)?;
            }
            Instruction::Istore1
            | Instruction::Lstore1
            | Instruction::Fstore1
            | Instruction::Dstore1
            | Instruction::Astore1 => {
                let v = frame.pop()?;
                frame.store_local(1, v)?;
            }
            Instruction::Istore2
            | Instruction::Lstore2
            | Instruction::Fstore2
            | Instruction::Dstore2
            | Instruction::Astore2 => {
                let v = frame.pop()?;
                frame.store_local(2, v)?;
            }
            Instruction::Istore3
            | Instruction::Lstore3
            | Instruction::Fstore3
            | Instruction::Dstore3
            | Instruction::Astore3 => {
                let v = frame.pop()?;
                frame.store_local(3, v)?;
            }
            Instruction::IstoreW(i)
            | Instruction::LstoreW(i)
            | Instruction::FstoreW(i)
            | Instruction::DstoreW(i)
            | Instruction::AstoreW(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }

            // ----------------------------------------------------------------
            // Stack manipulation
            // ----------------------------------------------------------------
            Instruction::Pop => {
                frame.pop()?;
            }
            Instruction::Pop2 => {
                frame.pop()?;
                frame.pop()?;
            }
            Instruction::Dup => {
                let v = frame.pop()?;
                frame.push(v)?;
                frame.push(v)?;
            }
            Instruction::DupX1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::DupX2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                let v4 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v4)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Swap => {
                let a = frame.pop()?;
                let b = frame.pop()?;
                frame.push(a)?;
                frame.push(b)?;
            }

            // ----------------------------------------------------------------
            // Integer arithmetic
            // ----------------------------------------------------------------
            Instruction::Iadd => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_add(b)))?;
            }
            Instruction::Isub => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_sub(b)))?;
            }
            Instruction::Imul => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_mul(b)))?;
            }
            Instruction::Idiv => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_div(b)))?;
            }
            Instruction::Irem => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_rem(b)))?;
            }
            Instruction::Ineg => {
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_neg()))?;
            }
            Instruction::Ishl => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shl((s & 0x1F) as u32)))?;
            }
            Instruction::Ishr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shr((s & 0x1F) as u32)))?;
            }
            Instruction::Iushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(((a as u32) >> (s as u32 & 0x1F)) as i32))?;
            }
            Instruction::Iand => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a & b))?;
            }
            Instruction::Ior => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a | b))?;
            }
            Instruction::Ixor => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a ^ b))?;
            }
            Instruction::Iinc { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }
            Instruction::IincW { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }

            // ----------------------------------------------------------------
            // Long arithmetic
            // ----------------------------------------------------------------
            Instruction::Ladd => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_add(b)))?;
            }
            Instruction::Lsub => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_sub(b)))?;
            }
            Instruction::Lmul => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_mul(b)))?;
            }
            Instruction::Ldiv => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_div(b)))?;
            }
            Instruction::Lrem => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_rem(b)))?;
            }
            Instruction::Lneg => {
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_neg()))?;
            }
            Instruction::Lshl => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shl((s & 0x3F) as u32)))?;
            }
            Instruction::Lshr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shr((s & 0x3F) as u32)))?;
            }
            Instruction::Lushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(((a as u64) >> (s as u32 & 0x3F)) as i64))?;
            }
            Instruction::Land => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a & b))?;
            }
            Instruction::Lor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a | b))?;
            }
            Instruction::Lxor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a ^ b))?;
            }
            Instruction::Lcmp => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                let r = match a.cmp(&b) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Float arithmetic
            // ----------------------------------------------------------------
            Instruction::Fadd => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a + b))?;
            }
            Instruction::Fsub => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a - b))?;
            }
            Instruction::Fmul => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a * b))?;
            }
            Instruction::Fdiv => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a / b))?;
            }
            Instruction::Frem => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a % b))?;
            }
            Instruction::Fneg => {
                let a = frame.pop_float()?;
                frame.push(Slot::Float(-a))?;
            }
            Instruction::Fcmpl | Instruction::Fcmpg => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                // Java semantics: exact bit equality for 0 check; NaN handled separately.
                #[allow(clippy::float_cmp)]
                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Fcmpg) {
                    1 // NaN result: Fcmpg pushes 1, Fcmpl pushes -1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Double arithmetic
            // ----------------------------------------------------------------
            Instruction::Dadd => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a + b))?;
            }
            Instruction::Dsub => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a - b))?;
            }
            Instruction::Dmul => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a * b))?;
            }
            Instruction::Ddiv => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a / b))?;
            }
            Instruction::Drem => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a % b))?;
            }
            Instruction::Dneg => {
                let a = frame.pop_double()?;
                frame.push(Slot::Double(-a))?;
            }
            Instruction::Dcmpl | Instruction::Dcmpg => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                // Java semantics: exact bit equality for 0 check; NaN handled separately.
                #[allow(clippy::float_cmp)]
                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Dcmpg) {
                    1 // NaN result: Dcmpg pushes 1, Dcmpl pushes -1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }

            // ----------------------------------------------------------------
            // Type conversions
            // ----------------------------------------------------------------
            Instruction::I2l => {
                let v = frame.pop_int()?;
                frame.push(Slot::Long(i64::from(v)))?;
            }
            Instruction::I2f => {
                let v = frame.pop_int()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2d => {
                let v = frame.pop_int()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::L2i => {
                let v = frame.pop_long()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::L2f => {
                let v = frame.pop_long()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::L2d => {
                let v = frame.pop_long()?;
                frame.push(Slot::Double(v as f64))?;
            }
            Instruction::F2i => {
                let v = frame.pop_float()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::F2l => {
                let v = frame.pop_float()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::F2d => {
                let v = frame.pop_float()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::D2i => {
                let v = frame.pop_double()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::D2l => {
                let v = frame.pop_double()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::D2f => {
                let v = frame.pop_double()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2b => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as i8)))?;
            }
            Instruction::I2c => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as u16)))?;
            }
            Instruction::I2s => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as i16)))?;
            }

            // ----------------------------------------------------------------
            // Returns
            // ----------------------------------------------------------------
            Instruction::Return => return Ok(None),
            Instruction::Ireturn => return Ok(Some(Slot::Int(frame.pop_int()?))),
            Instruction::Lreturn => return Ok(Some(Slot::Long(frame.pop_long()?))),
            Instruction::Freturn => return Ok(Some(Slot::Float(frame.pop_float()?))),
            Instruction::Dreturn => return Ok(Some(Slot::Double(frame.pop_double()?))),

            // ----------------------------------------------------------------
            // Branches
            // ----------------------------------------------------------------
            Instruction::Goto(offset) => jump!(*offset),
            Instruction::GotoW(offset) => jump!(*offset),

            Instruction::Ifeq(offset) => {
                if frame.pop_int()? == 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifne(offset) => {
                if frame.pop_int()? != 0 {
                    jump!(*offset);
                }
            }
            Instruction::Iflt(offset) => {
                if frame.pop_int()? < 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifge(offset) => {
                if frame.pop_int()? >= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifgt(offset) => {
                if frame.pop_int()? > 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifle(offset) => {
                if frame.pop_int()? <= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifnull(offset) => {
                if matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::Ifnonnull(offset) => {
                if !matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpeq(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpne(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a != b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmplt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a < b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpge(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a >= b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpgt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a > b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmple(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a <= b {
                    jump!(*offset);
                }
            }
            // Reference comparisons
            Instruction::IfAcmpeq(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfAcmpne(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a != b {
                    jump!(*offset);
                }
            }

            // ----------------------------------------------------------------
            // Array allocation
            // ----------------------------------------------------------------
            Instruction::Newarray(array_type) => {
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let class_name = match array_type {
                    ArrayType::Boolean => "[Z",
                    ArrayType::Char => "[C",
                    ArrayType::Float => "[F",
                    ArrayType::Double => "[D",
                    ArrayType::Byte => "[B",
                    ArrayType::Short => "[S",
                    ArrayType::Int => "[I",
                    ArrayType::Long => "[J",
                };
                let init_slot = match array_type {
                    ArrayType::Long => Slot::Long(0),
                    ArrayType::Float => Slot::Float(0.0),
                    ArrayType::Double => Slot::Double(0.0),
                    _ => Slot::Int(0),
                };
                let r = local_heap.len() as u64;
                local_heap.push((
                    class_name.to_string(),
                    vec![init_slot; count as usize],
                    None,
                ));
                frame.push(Slot::Reference(Some(r)))?;
            }
            Instruction::Anewarray(cp_idx) => {
                let element_type = resolve_class_name(cp, usize::from(cp_idx.0))?;
                let array_type = format!("[L{element_type};");
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let r = local_heap.len() as u64;
                local_heap.push((
                    array_type,
                    vec![Slot::Reference(None); count as usize],
                    None,
                ));
                frame.push(Slot::Reference(Some(r)))?;
            }
            Instruction::Arraylength => {
                let r = frame.pop_ref()?;
                let len = local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1
                    .len();
                frame.push(Slot::Int(len as i32))?;
            }

            // ---- Int array ----
            Instruction::Iaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_int()?;
                frame.push(Slot::Int(v))?;
            }
            Instruction::Iastore => {
                let val = frame.pop_int()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Long array ----
            Instruction::Laload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_long()?;
                frame.push(Slot::Long(v))?;
            }
            Instruction::Lastore => {
                let val = frame.pop_long()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Long(val);
            }

            // ---- Float array ----
            Instruction::Faload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_float()?;
                frame.push(Slot::Float(v))?;
            }
            Instruction::Fastore => {
                let val = frame.pop_float()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Float(val);
            }

            // ---- Double array ----
            Instruction::Daload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize].as_double()?;
                frame.push(Slot::Double(v))?;
            }
            Instruction::Dastore => {
                let val = frame.pop_double()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Double(val);
            }

            // ---- Reference array ----
            Instruction::Aaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = fields[idx_val as usize];
                frame.push(v)?;
            }
            Instruction::Aastore => {
                let val = frame.pop()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = val;
            }

            // ---- Byte/boolean array (stored as Int, truncated to i8) ----
            Instruction::Baload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as i8);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Bastore => {
                let val = i32::from(frame.pop_int()? as i8);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Char array (stored as Int, masked to u16) ----
            Instruction::Caload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as u16);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Castore => {
                let val = i32::from(frame.pop_int()? as u16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Short array (stored as Int, truncated to i16) ----
            Instruction::Saload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                let v = i32::from(fields[idx_val as usize].as_int()? as i16);
                frame.push(Slot::Int(v))?;
            }
            Instruction::Sastore => {
                let val = i32::from(frame.pop_int()? as i16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let fields = &mut local_heap
                    .get_mut(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .1;
                if idx_val < 0 || idx_val as usize >= fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: fields.len(),
                    });
                }
                fields[idx_val as usize] = Slot::Int(val);
            }

            // ---- Switch ----
            Instruction::Tableswitch {
                default,
                low,
                high,
                offsets,
            } => {
                let key = frame.pop_int()?;
                let offset = if key >= *low && key <= *high {
                    offsets[(key - low) as usize]
                } else {
                    *default
                };
                jump!(offset);
            }
            Instruction::Lookupswitch { default, pairs } => {
                let key = frame.pop_int()?;
                let offset = pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map_or(*default, |(_, off)| *off);
                jump!(offset);
            }

            // ---- checkcast / instanceof ----
            Instruction::Checkcast(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(slot)?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = resolve_class_name(cp, usize::from(cp_idx.0))?;
                        let actual = local_heap
                            .get(*r as usize)
                            .ok_or(VmError::InvalidRef { address: *r })?
                            .0
                            .clone();
                        if actual == target {
                            frame.push(slot)?;
                        } else {
                            return Err(VmError::ClassCastException {
                                from: actual,
                                to: target,
                            });
                        }
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }
            Instruction::Instanceof(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(Slot::Int(0))?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = resolve_class_name(cp, usize::from(cp_idx.0))?;
                        let actual = local_heap
                            .get(*r as usize)
                            .ok_or(VmError::InvalidRef { address: *r })?
                            .0
                            .clone();
                        if actual == target {
                            frame.push(Slot::Int(1))?;
                        } else {
                            frame.push(Slot::Int(0))?;
                        }
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }

            // ---- athrow ----
            Instruction::Athrow => {
                let r = frame.pop_ref()?;
                let class_name = local_heap
                    .get(r as usize)
                    .ok_or(VmError::InvalidRef { address: r })?
                    .0
                    .clone();
                return Err(VmError::JavaException { class_name });
            }

            // ---- monitor (no-op, single-threaded) ----
            Instruction::Monitorenter | Instruction::Monitorexit => {
                let _obj = frame.pop()?;
            }

            other => {
                return Err(VmError::Unimplemented {
                    mnemonic: other.mnemonic(),
                });
            }
        }

        idx += 1;
    }
}

/// Ensure a class is initialized. Runs `<clinit>` if present and not yet run.
///
/// Must be called before first active use of a class (new, getstatic, putstatic, invokestatic).
/// `triggered_by` names the class that caused this init (empty string for the entry-point class).
#[allow(clippy::used_underscore_binding, clippy::cast_possible_truncation)]
fn ensure_initialized(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    _triggered_by: &str,
) -> VmResult<()> {
    if registry.is_initialized(class_name) {
        return Ok(());
    }
    // Mark as initialized BEFORE running clinit to prevent infinite recursion.
    registry.mark_initialized(class_name);

    // Check if the class has a <clinit> method.
    let has_clinit = registry.get(class_name).is_ok_and(|ctx| {
        ctx.methods
            .iter()
            .any(|m| m.name == "<clinit>" && m.descriptor == "()V")
    });

    if has_clinit {
        // Run <clinit> by calling it through execute_class.
        #[cfg(feature = "telemetry")]
        #[allow(clippy::used_underscore_binding)]
        let _clinit_start = std::time::Instant::now();
        execute_class(
            registry,
            loader,
            heap,
            stdout,
            class_name,
            "<clinit>",
            "()V",
            &[],
        )?;
        #[cfg(feature = "telemetry")]
        registry.telemetry.class_init_dag.record(
            class_name,
            _triggered_by,
            _clinit_start.elapsed().as_nanos() as u64,
        );
    }
    Ok(())
}

/// Reusable pool of frame backing buffers.
///
/// Eliminates per-call `Vec<Slot>` allocation for locals and operand stack.
/// On a recursive workload the pool reaches steady state after the first
/// call-depth calls; all subsequent frames are pool hits with zero allocation.
///
/// The pool is private to a single `execute_class` invocation.
struct FramePool {
    free: Vec<(Vec<Slot>, Vec<Slot>)>,
}

impl FramePool {
    const fn new() -> Self {
        Self { free: Vec::new() }
    }

    /// Acquire a `(locals_buf, stack_buf)` pair.
    /// Returns a pooled pair if available, otherwise allocates fresh Vecs.
    fn acquire(&mut self) -> (Vec<Slot>, Vec<Slot>) {
        self.free.pop().unwrap_or_default()
    }

    /// Return buffers to the pool.
    /// `stack` must already be empty (guaranteed by `Frame::into_pool_bufs`).
    /// Caps pool at 256 entries to bound memory usage.
    fn release(&mut self, locals: Vec<Slot>, stack: Vec<Slot>) {
        // Cap at 256 entries — typical max call depth is well under 100; anything
        // beyond this is dead weight. Entries dropped here are freed to the allocator.
        if self.free.len() < 256 {
            self.free.push((locals, stack));
        }
    }
}

#[cfg(feature = "telemetry")]
#[allow(clippy::too_many_lines)]
const fn instr_name(instr: &duke_bytecode::Instruction) -> &'static str {
    use duke_bytecode::Instruction as I;
    match instr {
        I::Nop => "nop",
        I::AconstNull => "aconst_null",
        I::Iconst0 | I::Iconst1 | I::Iconst2 | I::Iconst3 | I::Iconst4 | I::Iconst5 => "iconst_n",
        I::IconstM1 => "iconst_m1",
        I::Lconst0 | I::Lconst1 => "lconst_n",
        I::Fconst0 | I::Fconst1 | I::Fconst2 => "fconst_n",
        I::Dconst0 | I::Dconst1 => "dconst_n",
        I::Bipush(_) => "bipush",
        I::Sipush(_) => "sipush",
        I::Ldc(_) | I::LdcW(_) | I::Ldc2W(_) => "ldc",
        I::Iload(_) | I::Iload0 | I::Iload1 | I::Iload2 | I::Iload3 | I::IloadW(_) => "iload",
        I::Lload(_) | I::Lload0 | I::Lload1 | I::Lload2 | I::Lload3 | I::LloadW(_) => "lload",
        I::Fload(_) | I::Fload0 | I::Fload1 | I::Fload2 | I::Fload3 | I::FloadW(_) => "fload",
        I::Dload(_) | I::Dload0 | I::Dload1 | I::Dload2 | I::Dload3 | I::DloadW(_) => "dload",
        I::Aload(_) | I::Aload0 | I::Aload1 | I::Aload2 | I::Aload3 | I::AloadW(_) => "aload",
        I::Istore(_) | I::Istore0 | I::Istore1 | I::Istore2 | I::Istore3 | I::IstoreW(_) => {
            "istore"
        }
        I::Lstore(_) | I::Lstore0 | I::Lstore1 | I::Lstore2 | I::Lstore3 | I::LstoreW(_) => {
            "lstore"
        }
        I::Fstore(_) | I::Fstore0 | I::Fstore1 | I::Fstore2 | I::Fstore3 | I::FstoreW(_) => {
            "fstore"
        }
        I::Dstore(_) | I::Dstore0 | I::Dstore1 | I::Dstore2 | I::Dstore3 | I::DstoreW(_) => {
            "dstore"
        }
        I::Astore(_) | I::Astore0 | I::Astore1 | I::Astore2 | I::Astore3 | I::AstoreW(_) => {
            "astore"
        }
        I::Iaload => "iaload",
        I::Laload => "laload",
        I::Faload => "faload",
        I::Daload => "daload",
        I::Aaload => "aaload",
        I::Baload => "baload",
        I::Caload => "caload",
        I::Saload => "saload",
        I::Iastore => "iastore",
        I::Lastore => "lastore",
        I::Fastore => "fastore",
        I::Dastore => "dastore",
        I::Aastore => "aastore",
        I::Bastore => "bastore",
        I::Castore => "castore",
        I::Sastore => "sastore",
        I::Pop => "pop",
        I::Pop2 => "pop2",
        I::Dup => "dup",
        I::DupX1 => "dup_x1",
        I::DupX2 => "dup_x2",
        I::Dup2 => "dup2",
        I::Dup2X1 => "dup2_x1",
        I::Dup2X2 => "dup2_x2",
        I::Swap => "swap",
        I::Iadd => "iadd",
        I::Ladd => "ladd",
        I::Fadd => "fadd",
        I::Dadd => "dadd",
        I::Isub => "isub",
        I::Lsub => "lsub",
        I::Fsub => "fsub",
        I::Dsub => "dsub",
        I::Imul => "imul",
        I::Lmul => "lmul",
        I::Fmul => "fmul",
        I::Dmul => "dmul",
        I::Idiv => "idiv",
        I::Ldiv => "ldiv",
        I::Fdiv => "fdiv",
        I::Ddiv => "ddiv",
        I::Irem => "irem",
        I::Lrem => "lrem",
        I::Frem => "frem",
        I::Drem => "drem",
        I::Ineg => "ineg",
        I::Lneg => "lneg",
        I::Fneg => "fneg",
        I::Dneg => "dneg",
        I::Ishl => "ishl",
        I::Lshl => "lshl",
        I::Ishr => "ishr",
        I::Lshr => "lshr",
        I::Iushr => "iushr",
        I::Lushr => "lushr",
        I::Iand => "iand",
        I::Land => "land",
        I::Ior => "ior",
        I::Lor => "lor",
        I::Ixor => "ixor",
        I::Lxor => "lxor",
        I::Iinc { .. } | I::IincW { .. } => "iinc",
        I::I2l => "i2l",
        I::I2f => "i2f",
        I::I2d => "i2d",
        I::L2i => "l2i",
        I::L2f => "l2f",
        I::L2d => "l2d",
        I::F2i => "f2i",
        I::F2l => "f2l",
        I::F2d => "f2d",
        I::D2i => "d2i",
        I::D2l => "d2l",
        I::D2f => "d2f",
        I::I2b => "i2b",
        I::I2c => "i2c",
        I::I2s => "i2s",
        I::Lcmp => "lcmp",
        I::Fcmpl => "fcmpl",
        I::Fcmpg => "fcmpg",
        I::Dcmpl => "dcmpl",
        I::Dcmpg => "dcmpg",
        I::Ifeq(_) | I::Ifne(_) | I::Iflt(_) | I::Ifge(_) | I::Ifgt(_) | I::Ifle(_) => "if_<cond>",
        I::IfIcmpeq(_)
        | I::IfIcmpne(_)
        | I::IfIcmplt(_)
        | I::IfIcmpge(_)
        | I::IfIcmpgt(_)
        | I::IfIcmple(_) => "if_icmp<cond>",
        I::IfAcmpeq(_) | I::IfAcmpne(_) => "if_acmp<cond>",
        I::Goto(_) | I::GotoW(_) => "goto",
        I::Jsr(_) | I::JsrW(_) => "jsr",
        I::Ret(_) | I::RetW(_) => "ret",
        I::Tableswitch { .. } => "tableswitch",
        I::Lookupswitch { .. } => "lookupswitch",
        I::Ireturn => "ireturn",
        I::Lreturn => "lreturn",
        I::Freturn => "freturn",
        I::Dreturn => "dreturn",
        I::Areturn => "areturn",
        I::Return => "return",
        I::Getstatic(_) => "getstatic",
        I::Putstatic(_) => "putstatic",
        I::Getfield(_) => "getfield",
        I::Putfield(_) => "putfield",
        I::Invokevirtual(_) => "invokevirtual",
        I::Invokespecial(_) => "invokespecial",
        I::Invokestatic(_) => "invokestatic",
        I::Invokeinterface { .. } => "invokeinterface",
        I::Invokedynamic(_) => "invokedynamic",
        I::New(_) => "new",
        I::Newarray(_) => "newarray",
        I::Anewarray(_) => "anewarray",
        I::Arraylength => "arraylength",
        I::Athrow => "athrow",
        I::Checkcast(_) => "checkcast",
        I::Instanceof(_) => "instanceof",
        I::Monitorenter => "monitorenter",
        I::Monitorexit => "monitorexit",
        I::Multianewarray { .. } => "multianewarray",
        I::Ifnull(_) | I::Ifnonnull(_) => "ifnull/nonnull",
    }
}

/// Execute a static method by name within a loaded class context.
///
/// Supports `invokestatic` calls between methods in the same class.
///
/// # Errors
/// Returns [`VmError`] on execution faults or if `method_name`/`descriptor`
/// are not found in `ctx`.
///
/// # Panics
/// Panics on internal invariant violations, such as a `Class` CP entry whose
/// name index refers to a non-`Utf8` entry (indicates a malformed class file).
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::manual_let_else,
    clippy::single_match,
    clippy::single_match_else,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::used_underscore_binding
)]
pub fn execute_class(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    args: &[Slot],
) -> VmResult<Option<Slot>> {
    macro_rules! create_invoke_cb {
        ($registry:ident, $loader:ident) => {
            |heap: &mut duke_gc::Heap,
             output: &mut dyn std::io::Write,
             class: &str,
             method: &str,
             desc: &str,
             cb_args: Vec<Slot>|
             -> VmResult<Option<Slot>> {
                execute_class(
                    $registry, $loader, heap, output, class, method, desc, &cb_args,
                )
            }
        };
    }

    // Fast path: if a native handler is registered for this class/method/descriptor,
    // dispatch it directly without requiring a ClassContext in the registry.
    // This handles both Simple natives and Callback natives at the top-level call site.
    match registry
        .natives_mut()
        .get_kind(class_name, method_name, descriptor)
    {
        Some(HandlerKind::Simple(h)) => {
            return h(args, heap, stdout);
        }
        Some(HandlerKind::Callback(h)) => {
            let mut invoke_cb = create_invoke_cb!(registry, loader);
            return h(args, heap, stdout, &mut invoke_cb);
        }
        None => {}
    }

    // Find entry method.
    let entry_idx = {
        let ctx = registry.get(class_name)?;
        ctx.methods
            .iter()
            .position(|m| m.name == method_name && m.descriptor == descriptor)
            .ok_or_else(|| VmError::MethodNotFound {
                name: method_name.to_string(),
                descriptor: descriptor.to_string(),
            })?
    };

    // Ensure the entry class has been initialized (<clinit> run).
    ensure_initialized(registry, loader, heap, stdout, class_name, "")?;

    let mut current_class = class_name.to_string();
    #[cfg(feature = "telemetry")]
    let mut current_method = method_name.to_string();
    let mut call_stack: Vec<CallFrame> = Vec::new();
    let mut frame_pool = FramePool::new();
    // Dispatch cache: caller_class_name -> cp_idx -> (callee_class_name, method_idx, arg_count).
    // Nested map lets the outer lookup borrow current_class as &str (no clone on cache hits).
    // Eliminates repeated CP 3-level walk + linear method search for repeat static/special call sites.
    let mut dispatch_cache: HashMap<String, HashMap<u16, (String, usize, usize)>> = HashMap::new();
    let mut method_idx = entry_idx;
    let mut pc_to_idx = {
        let ctx = registry.get(&current_class)?;
        std::sync::Arc::clone(&ctx.methods[method_idx].pc_to_idx)
    };
    let mut frame = {
        let ctx = registry.get(&current_class)?;
        Frame::new(
            usize::from(ctx.methods[method_idx].max_stack),
            usize::from(ctx.methods[method_idx].max_locals),
            args.to_vec(),
        )?
    };
    let mut idx: usize = 0;
    let mut string_intern: HashMap<usize, u64> = HashMap::new();

    let mut instructions = {
        let ctx = registry.get(&current_class)?;
        std::sync::Arc::clone(&ctx.methods[method_idx].instructions)
    };

    loop {
        let (pc, instr) = {
            let Some(&(pc, ref instr)) = instructions.get(idx) else {
                return Err(VmError::FellOffEnd);
            };
            (pc, instr.clone())
        };

        macro_rules! jump {
            ($offset:expr) => {{
                let target = (pc as i64).wrapping_add(i64::from($offset)) as usize;
                idx = *pc_to_idx
                    .get(&target)
                    .ok_or(VmError::InvalidBranchTarget { pc: target })?;
                continue;
            }};
        }

        // Helper macro for return instructions: pop call stack or return to Rust.
        macro_rules! do_return {
            ($val:expr) => {{
                match call_stack.pop() {
                    None => return Ok($val),
                    Some(caller) => {
                        let ret_val = $val;
                        // Recycle callee frame buffers before overwriting `frame`.
                        let old = std::mem::replace(&mut frame, caller.frame);
                        let (l, s) = old.into_pool_bufs();
                        frame_pool.release(l, s);
                        method_idx = caller.method_idx;
                        pc_to_idx = caller.pc_to_idx;
                        idx = caller.resume_idx;
                        current_class = caller.class_name;
                        instructions = std::sync::Arc::clone(
                            &registry.get(&current_class)?.methods[method_idx].instructions,
                        );
                        #[cfg(feature = "telemetry")]
                        {
                            current_method = registry
                                .get(&current_class)
                                .map(|c| {
                                    c.methods
                                        .get(method_idx)
                                        .map(|m| m.name.clone())
                                        .unwrap_or_default()
                                })
                                .unwrap_or_default();
                        }
                        if let Some(v) = ret_val {
                            frame.push(v)?;
                        }
                        continue;
                    }
                }
            }};
        }

        macro_rules! propagate_java_exception {
            ($exc_class_name:expr, $exception_ref:expr, $throw_pc:expr) => {{
                let exc_class_name = $exc_class_name;
                let exception_ref = $exception_ref;

                #[cfg(feature = "telemetry")]
                let _telem_exc_event_idx = registry.telemetry.exception_flow.record_throw(
                    &exc_class_name,
                    &current_class,
                    &current_method,
                    $throw_pc,
                );

                // Clone the exception table to release the borrow on registry,
                // so find_exception_handler can use &mut registry for hierarchy checks.
                let exc_table = registry.get(&current_class)?.methods[method_idx]
                    .exception_table
                    .iter()
                    .map(|e| ExceptionEntry {
                        start_pc: e.start_pc,
                        end_pc: e.end_pc,
                        handler_pc: e.handler_pc,
                        catch_type: e.catch_type.clone(),
                    })
                    .collect::<Vec<_>>();
                let handler = find_exception_handler(
                    &exc_table,
                    $throw_pc,
                    &exc_class_name,
                    registry,
                    loader,
                );
                if let Some(handler_pc) = handler {
                    #[cfg(feature = "telemetry")]
                    registry.telemetry.exception_flow.record_catch(
                        _telem_exc_event_idx,
                        &current_class,
                        &current_method,
                        handler_pc as usize,
                    );
                    frame.clear_stack();
                    frame.push(Slot::Reference(Some(exception_ref)))?;
                    idx = *pc_to_idx.get(&(handler_pc as usize)).ok_or(
                        VmError::InvalidBranchTarget {
                            pc: handler_pc as usize,
                        },
                    )?;
                    continue;
                }

                loop {
                    match call_stack.pop() {
                        None => {
                            return Err(VmError::JavaException {
                                class_name: exc_class_name,
                            });
                        }
                        Some(caller) => {
                            // Recycle callee frame buffers before overwriting `frame` —
                            // mirrors the do_return! pattern to avoid a pool leak.
                            let old = std::mem::replace(&mut frame, caller.frame);
                            let (l, s) = old.into_pool_bufs();
                            frame_pool.release(l, s);
                            method_idx = caller.method_idx;
                            pc_to_idx = caller.pc_to_idx;
                            current_class = caller.class_name;
                            instructions = std::sync::Arc::clone(
                                &registry.get(&current_class)?.methods[method_idx].instructions,
                            );
                            #[cfg(feature = "telemetry")]
                            {
                                current_method = registry
                                    .get(&current_class)
                                    .map(|c| {
                                        c.methods
                                            .get(method_idx)
                                            .map(|m| m.name.clone())
                                            .unwrap_or_default()
                                    })
                                    .unwrap_or_default();
                            }

                            let (caller_exc_table, caller_pc) = {
                                let ctx = registry.get(&current_class)?;
                                let cpc = if caller.resume_idx > 0 {
                                    ctx.methods[method_idx].instructions[caller.resume_idx - 1].0
                                } else {
                                    0
                                };
                                let tbl = ctx.methods[method_idx]
                                    .exception_table
                                    .iter()
                                    .map(|e| ExceptionEntry {
                                        start_pc: e.start_pc,
                                        end_pc: e.end_pc,
                                        handler_pc: e.handler_pc,
                                        catch_type: e.catch_type.clone(),
                                    })
                                    .collect::<Vec<_>>();
                                (tbl, cpc)
                            };
                            let handler = find_exception_handler(
                                &caller_exc_table,
                                caller_pc,
                                &exc_class_name,
                                registry,
                                loader,
                            );

                            if let Some(handler_pc) = handler {
                                #[cfg(feature = "telemetry")]
                                registry.telemetry.exception_flow.record_catch(
                                    _telem_exc_event_idx,
                                    &current_class,
                                    &current_method,
                                    handler_pc as usize,
                                );
                                frame.clear_stack();
                                frame.push(Slot::Reference(Some(exception_ref)))?;
                                idx = *pc_to_idx.get(&(handler_pc as usize)).ok_or(
                                    VmError::InvalidBranchTarget {
                                        pc: handler_pc as usize,
                                    },
                                )?;
                                break;
                            }
                        }
                    }
                }
                continue;
            }};
        }

        // Telemetry: capture opcode name and start time before dispatch.
        // Arms that use `continue` (branches, invokes) will skip the post-match
        // recording for that iteration — timing is approximate for those opcodes.
        #[cfg(feature = "telemetry")]
        #[allow(clippy::used_underscore_binding)]
        let (_telem_name, _telem_pc, _telem_start) = {
            let name = instr_name(&instr);
            let pc_val = pc;
            (name, pc_val, std::time::Instant::now())
        };

        match &instr {
            // ---- invokestatic ----
            Instruction::Invokestatic(cp_idx) => {
                if let Some(&(ref cached_cls, cached_idx, cached_ac)) = dispatch_cache
                    .get(current_class.as_str())
                    .and_then(|m| m.get(&cp_idx.0))
                {
                    // Fast path: cache hit — skip CP walk and method search.
                    let (callee_pc_to_idx, callee_frame) = {
                        let ctx = registry.get(cached_cls)?;
                        let max_locals = usize::from(ctx.methods[cached_idx].max_locals);
                        let max_stack = usize::from(ctx.methods[cached_idx].max_stack);
                        let pci = std::sync::Arc::clone(&ctx.methods[cached_idx].pc_to_idx);
                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                        locals_buf.resize(max_locals, Slot::Int(0));
                        if cached_ac > max_locals {
                            return Err(VmError::LocalOutOfBounds {
                                index: cached_ac,
                                max_locals,
                            });
                        }
                        for i in (0..cached_ac).rev() {
                            locals_buf[i] = frame.pop()?;
                        }
                        let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                        (pci, f)
                    };
                    let callee_class = cached_cls.clone();
                    let callee_idx = cached_idx;
                    call_stack.push(CallFrame {
                        frame,
                        method_idx,
                        pc_to_idx,
                        resume_idx: idx + 1,
                        class_name: current_class.clone(),
                    });
                    frame = callee_frame;
                    method_idx = callee_idx;
                    pc_to_idx = callee_pc_to_idx;
                    current_class = callee_class;
                    instructions = std::sync::Arc::clone(
                        &registry.get(&current_class)?.methods[method_idx].instructions,
                    );
                    #[cfg(feature = "telemetry")]
                    {
                        current_method = registry
                            .get(&current_class)
                            .map(|c| {
                                c.methods
                                    .get(method_idx)
                                    .map(|m| m.name.clone())
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default();
                    }
                    idx = 0;
                    continue;
                }
                // Slow path: full CP resolution + method search.
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                registry.ensure_loaded(&callee_class, loader)?;
                ensure_initialized(
                    registry,
                    loader,
                    heap,
                    stdout,
                    &callee_class,
                    &current_class,
                )?;
                let callee_idx = {
                    let ctx = registry.get(&callee_class)?;
                    ctx.methods
                        .iter()
                        .position(|m| m.name == callee_name && m.descriptor == callee_desc)
                };
                match callee_idx {
                    Some(callee_idx) => {
                        let arg_count = parse_arg_count(&callee_desc);
                        dispatch_cache
                            .entry(current_class.clone())
                            .or_default()
                            .insert(cp_idx.0, (callee_class.clone(), callee_idx, arg_count));
                        let (callee_pc_to_idx, callee_frame) = {
                            let ctx = registry.get(&callee_class)?;
                            let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                            let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                            let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                            let (mut locals_buf, stack_buf) = frame_pool.acquire();
                            locals_buf.resize(max_locals, Slot::Int(0));
                            if arg_count > max_locals {
                                return Err(VmError::LocalOutOfBounds {
                                    index: arg_count,
                                    max_locals,
                                });
                            }
                            for i in (0..arg_count).rev() {
                                locals_buf[i] = frame.pop()?;
                            }
                            let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                            (pci, f)
                        };
                        call_stack.push(CallFrame {
                            frame,
                            method_idx,
                            pc_to_idx,
                            resume_idx: idx + 1,
                            class_name: current_class.clone(),
                        });
                        frame = callee_frame;
                        method_idx = callee_idx;
                        pc_to_idx = callee_pc_to_idx;
                        current_class = callee_class;
                        instructions = std::sync::Arc::clone(
                            &registry.get(&current_class)?.methods[method_idx].instructions,
                        );
                        #[cfg(feature = "telemetry")]
                        {
                            current_method = registry
                                .get(&current_class)
                                .map(|c| {
                                    c.methods
                                        .get(method_idx)
                                        .map(|m| m.name.clone())
                                        .unwrap_or_default()
                                })
                                .unwrap_or_default();
                        }
                        idx = 0;
                        continue;
                    }
                    None => {
                        // Check native registry before erroring.
                        let handler_kind = registry.natives_mut().get_kind(
                            &callee_class,
                            &callee_name,
                            &callee_desc,
                        );
                        match handler_kind {
                            Some(HandlerKind::Simple(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                let mut native_args = vec![Slot::Int(0); arg_count];
                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let result = handler(&native_args, heap, stdout);
                                #[cfg(feature = "telemetry")]
                                registry.telemetry.native_boundary.record_call(
                                    &callee_class,
                                    &callee_name,
                                    _native_start.elapsed().as_nanos() as u64,
                                    result.is_err(),
                                );
                                let result = match result {
                                    Ok(result) => result,
                                    Err(VmError::JavaException { class_name }) => {
                                        let exception_ref = materialize_java_exception_object(
                                            registry,
                                            loader,
                                            heap,
                                            &class_name,
                                        )?;
                                        propagate_java_exception!(class_name, exception_ref, pc);
                                    }
                                    Err(err) => return Err(err),
                                };
                                if let Some(val) = result {
                                    frame.push(val)?;
                                }
                                idx += 1;
                                continue;
                            }
                            Some(HandlerKind::Callback(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                let mut native_args = vec![Slot::Int(0); arg_count];
                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut invoke_cb = create_invoke_cb!(registry, loader);
                                let result = handler(&native_args, heap, stdout, &mut invoke_cb);
                                #[cfg(feature = "telemetry")]
                                registry.telemetry.native_boundary.record_call(
                                    &callee_class,
                                    &callee_name,
                                    _native_start.elapsed().as_nanos() as u64,
                                    result.is_err(),
                                );
                                let result = match result {
                                    Ok(result) => result,
                                    Err(VmError::JavaException { class_name }) => {
                                        let exception_ref = materialize_java_exception_object(
                                            registry,
                                            loader,
                                            heap,
                                            &class_name,
                                        )?;
                                        propagate_java_exception!(class_name, exception_ref, pc);
                                    }
                                    Err(err) => return Err(err),
                                };
                                if let Some(val) = result {
                                    frame.push(val)?;
                                }
                                idx += 1;
                                continue;
                            }
                            None => {
                                return Err(VmError::MethodNotFound {
                                    name: callee_name,
                                    descriptor: callee_desc,
                                });
                            }
                        }
                    }
                }
            }

            // ---- returns ----
            Instruction::Return => do_return!(None),
            Instruction::Ireturn => {
                let v = frame.pop_int()?;
                do_return!(Some(Slot::Int(v)));
            }
            Instruction::Lreturn => {
                let v = frame.pop_long()?;
                do_return!(Some(Slot::Long(v)));
            }
            Instruction::Freturn => {
                let v = frame.pop_float()?;
                do_return!(Some(Slot::Float(v)));
            }
            Instruction::Dreturn => {
                let v = frame.pop_double()?;
                do_return!(Some(Slot::Double(v)));
            }

            // ---- all other instructions: same as execute() ----
            Instruction::Nop => {}
            Instruction::AconstNull => frame.push(Slot::Reference(None))?,
            Instruction::IconstM1 => frame.push(Slot::Int(-1))?,
            Instruction::Iconst0 => frame.push(Slot::Int(0))?,
            Instruction::Iconst1 => frame.push(Slot::Int(1))?,
            Instruction::Iconst2 => frame.push(Slot::Int(2))?,
            Instruction::Iconst3 => frame.push(Slot::Int(3))?,
            Instruction::Iconst4 => frame.push(Slot::Int(4))?,
            Instruction::Iconst5 => frame.push(Slot::Int(5))?,
            Instruction::Lconst0 => frame.push(Slot::Long(0))?,
            Instruction::Lconst1 => frame.push(Slot::Long(1))?,
            Instruction::Fconst0 => frame.push(Slot::Float(0.0))?,
            Instruction::Fconst1 => frame.push(Slot::Float(1.0))?,
            Instruction::Fconst2 => frame.push(Slot::Float(2.0))?,
            Instruction::Dconst0 => frame.push(Slot::Double(0.0))?,
            Instruction::Dconst1 => frame.push(Slot::Double(1.0))?,
            Instruction::Bipush(v) => frame.push(Slot::Int(i32::from(*v)))?,
            Instruction::Sipush(v) => frame.push(Slot::Int(i32::from(*v)))?,
            Instruction::Ldc(raw_idx) => {
                let cp_idx = usize::from(*raw_idx);
                let string_info = {
                    let ctx = registry.get(&current_class)?;
                    if let Some(CpEntry::String { string_index }) =
                        ctx.constant_pool.get(cp_idx).and_then(|e| e.as_ref())
                    {
                        let si = string_index.0 as usize;
                        let s = match ctx.constant_pool.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        Some(s)
                    } else {
                        None
                    }
                };
                if let Some(s) = string_info {
                    let r = if let Some(&cached) = string_intern.get(&cp_idx) {
                        cached
                    } else {
                        let r = heap.allocate_string(s);
                        string_intern.insert(cp_idx, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    // Check for Class constant
                    let class_info = {
                        let ctx = registry.get(&current_class)?;
                        if let Some(CpEntry::Class { name_index }) =
                            ctx.constant_pool.get(cp_idx).and_then(|e| e.as_ref())
                        {
                            match ctx
                                .constant_pool
                                .get(name_index.0 as usize)
                                .and_then(|e| e.as_ref())
                            {
                                Some(CpEntry::Utf8(s)) => Some(s.clone()),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    };
                    if let Some(class_name) = class_info {
                        // Intern class literals using offset key to avoid collision with String interning
                        let intern_key = cp_idx + 100_000;
                        let r = if let Some(&cached) = string_intern.get(&intern_key) {
                            cached
                        } else {
                            let r = heap.allocate("java/lang/Class".to_string(), 0);
                            heap.get_mut(r).unwrap().string_value = Some(class_name);
                            string_intern.insert(intern_key, r);
                            r
                        };
                        frame.push(Slot::Reference(Some(r)))?;
                    } else {
                        let ctx = registry.get(&current_class)?;
                        ldc_push(&mut frame, &ctx.constant_pool, cp_idx)?;
                    }
                }
            }
            Instruction::LdcW(cp_idx) | Instruction::Ldc2W(cp_idx) => {
                let idx_val = usize::from(cp_idx.0);
                let string_info = {
                    let ctx = registry.get(&current_class)?;
                    if let Some(CpEntry::String { string_index }) =
                        ctx.constant_pool.get(idx_val).and_then(|e| e.as_ref())
                    {
                        let si = string_index.0 as usize;
                        let s = match ctx.constant_pool.get(si).and_then(|e| e.as_ref()) {
                            Some(CpEntry::Utf8(s)) => s.clone(),
                            _ => return Err(VmError::InvalidCpIndex { index: si }),
                        };
                        Some(s)
                    } else {
                        None
                    }
                };
                if let Some(s) = string_info {
                    let r = if let Some(&cached) = string_intern.get(&idx_val) {
                        cached
                    } else {
                        let r = heap.allocate_string(s);
                        string_intern.insert(idx_val, r);
                        r
                    };
                    frame.push(Slot::Reference(Some(r)))?;
                } else {
                    // Check for Class constant
                    let class_info = {
                        let ctx = registry.get(&current_class)?;
                        if let Some(CpEntry::Class { name_index }) =
                            ctx.constant_pool.get(idx_val).and_then(|e| e.as_ref())
                        {
                            match ctx
                                .constant_pool
                                .get(name_index.0 as usize)
                                .and_then(|e| e.as_ref())
                            {
                                Some(CpEntry::Utf8(s)) => Some(s.clone()),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    };
                    if let Some(class_name) = class_info {
                        // Intern class literals using offset key to avoid collision with String interning
                        let intern_key = idx_val + 100_000;
                        let r = if let Some(&cached) = string_intern.get(&intern_key) {
                            cached
                        } else {
                            let r = heap.allocate("java/lang/Class".to_string(), 0);
                            heap.get_mut(r).unwrap().string_value = Some(class_name);
                            string_intern.insert(intern_key, r);
                            r
                        };
                        frame.push(Slot::Reference(Some(r)))?;
                    } else {
                        let ctx = registry.get(&current_class)?;
                        ldc_push(&mut frame, &ctx.constant_pool, idx_val)?;
                    }
                }
            }
            Instruction::Iload(i)
            | Instruction::Lload(i)
            | Instruction::Fload(i)
            | Instruction::Dload(i)
            | Instruction::Aload(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }
            Instruction::Iload0
            | Instruction::Lload0
            | Instruction::Fload0
            | Instruction::Dload0
            | Instruction::Aload0 => {
                let s = frame.load_local(0)?;
                frame.push(s)?;
            }
            Instruction::Iload1
            | Instruction::Lload1
            | Instruction::Fload1
            | Instruction::Dload1
            | Instruction::Aload1 => {
                let s = frame.load_local(1)?;
                frame.push(s)?;
            }
            Instruction::Iload2
            | Instruction::Lload2
            | Instruction::Fload2
            | Instruction::Dload2
            | Instruction::Aload2 => {
                let s = frame.load_local(2)?;
                frame.push(s)?;
            }
            Instruction::Iload3
            | Instruction::Lload3
            | Instruction::Fload3
            | Instruction::Dload3
            | Instruction::Aload3 => {
                let s = frame.load_local(3)?;
                frame.push(s)?;
            }
            Instruction::IloadW(i)
            | Instruction::LloadW(i)
            | Instruction::FloadW(i)
            | Instruction::DloadW(i)
            | Instruction::AloadW(i) => {
                let slot = frame.load_local(usize::from(*i))?;
                frame.push(slot)?;
            }
            Instruction::Istore(i)
            | Instruction::Lstore(i)
            | Instruction::Fstore(i)
            | Instruction::Dstore(i)
            | Instruction::Astore(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }
            Instruction::Istore0
            | Instruction::Lstore0
            | Instruction::Fstore0
            | Instruction::Dstore0
            | Instruction::Astore0 => {
                let v = frame.pop()?;
                frame.store_local(0, v)?;
            }
            Instruction::Istore1
            | Instruction::Lstore1
            | Instruction::Fstore1
            | Instruction::Dstore1
            | Instruction::Astore1 => {
                let v = frame.pop()?;
                frame.store_local(1, v)?;
            }
            Instruction::Istore2
            | Instruction::Lstore2
            | Instruction::Fstore2
            | Instruction::Dstore2
            | Instruction::Astore2 => {
                let v = frame.pop()?;
                frame.store_local(2, v)?;
            }
            Instruction::Istore3
            | Instruction::Lstore3
            | Instruction::Fstore3
            | Instruction::Dstore3
            | Instruction::Astore3 => {
                let v = frame.pop()?;
                frame.store_local(3, v)?;
            }
            Instruction::IstoreW(i)
            | Instruction::LstoreW(i)
            | Instruction::FstoreW(i)
            | Instruction::DstoreW(i)
            | Instruction::AstoreW(i) => {
                let v = frame.pop()?;
                frame.store_local(usize::from(*i), v)?;
            }
            Instruction::Pop => {
                frame.pop()?;
            }
            Instruction::Pop2 => {
                frame.pop()?;
                frame.pop()?;
            }
            Instruction::Dup => {
                let v = frame.pop()?;
                frame.push(v)?;
                frame.push(v)?;
            }
            Instruction::DupX1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::DupX2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                let v4 = frame.pop()?;
                frame.push(v2)?;
                frame.push(v1)?;
                frame.push(v4)?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Swap => {
                let a = frame.pop()?;
                let b = frame.pop()?;
                frame.push(a)?;
                frame.push(b)?;
            }
            Instruction::Iadd => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_add(b)))?;
            }
            Instruction::Isub => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_sub(b)))?;
            }
            Instruction::Imul => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_mul(b)))?;
            }
            Instruction::Idiv => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_div(b)))?;
            }
            Instruction::Irem => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Int(a.wrapping_rem(b)))?;
            }
            Instruction::Ineg => {
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_neg()))?;
            }
            Instruction::Ishl => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shl((s & 0x1F) as u32)))?;
            }
            Instruction::Ishr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a.wrapping_shr((s & 0x1F) as u32)))?;
            }
            Instruction::Iushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(((a as u32) >> (s as u32 & 0x1F)) as i32))?;
            }
            Instruction::Iand => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a & b))?;
            }
            Instruction::Ior => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a | b))?;
            }
            Instruction::Ixor => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                frame.push(Slot::Int(a ^ b))?;
            }
            Instruction::Iinc { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }
            Instruction::IincW { index, value } => {
                let v = frame.load_local(usize::from(*index))?.as_int()?;
                frame.store_local(
                    usize::from(*index),
                    Slot::Int(v.wrapping_add(i32::from(*value))),
                )?;
            }
            Instruction::Ladd => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_add(b)))?;
            }
            Instruction::Lsub => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_sub(b)))?;
            }
            Instruction::Lmul => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_mul(b)))?;
            }
            Instruction::Ldiv => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_div(b)))?;
            }
            Instruction::Lrem => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                if b == 0 {
                    return Err(VmError::DivisionByZero);
                }
                frame.push(Slot::Long(a.wrapping_rem(b)))?;
            }
            Instruction::Lneg => {
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_neg()))?;
            }
            Instruction::Lshl => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shl((s & 0x3F) as u32)))?;
            }
            Instruction::Lshr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a.wrapping_shr((s & 0x3F) as u32)))?;
            }
            Instruction::Lushr => {
                let s = frame.pop_int()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(((a as u64) >> (s as u32 & 0x3F)) as i64))?;
            }
            Instruction::Land => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a & b))?;
            }
            Instruction::Lor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a | b))?;
            }
            Instruction::Lxor => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                frame.push(Slot::Long(a ^ b))?;
            }
            Instruction::Lcmp => {
                let b = frame.pop_long()?;
                let a = frame.pop_long()?;
                let r = match a.cmp(&b) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                frame.push(Slot::Int(r))?;
            }
            Instruction::Fadd => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a + b))?;
            }
            Instruction::Fsub => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a - b))?;
            }
            Instruction::Fmul => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a * b))?;
            }
            Instruction::Fdiv => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a / b))?;
            }
            Instruction::Frem => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                frame.push(Slot::Float(a % b))?;
            }
            Instruction::Fneg => {
                let a = frame.pop_float()?;
                frame.push(Slot::Float(-a))?;
            }
            Instruction::Fcmpl | Instruction::Fcmpg => {
                let b = frame.pop_float()?;
                let a = frame.pop_float()?;
                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Fcmpg) {
                    1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }
            Instruction::Dadd => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a + b))?;
            }
            Instruction::Dsub => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a - b))?;
            }
            Instruction::Dmul => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a * b))?;
            }
            Instruction::Ddiv => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a / b))?;
            }
            Instruction::Drem => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                frame.push(Slot::Double(a % b))?;
            }
            Instruction::Dneg => {
                let a = frame.pop_double()?;
                frame.push(Slot::Double(-a))?;
            }
            Instruction::Dcmpl | Instruction::Dcmpg => {
                let b = frame.pop_double()?;
                let a = frame.pop_double()?;
                let r = if a > b {
                    1
                } else if a < b {
                    -1
                } else if a == b {
                    0
                } else if matches!(instr, Instruction::Dcmpg) {
                    1
                } else {
                    -1
                };
                frame.push(Slot::Int(r))?;
            }
            Instruction::I2l => {
                let v = frame.pop_int()?;
                frame.push(Slot::Long(i64::from(v)))?;
            }
            Instruction::I2f => {
                let v = frame.pop_int()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2d => {
                let v = frame.pop_int()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::L2i => {
                let v = frame.pop_long()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::L2f => {
                let v = frame.pop_long()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::L2d => {
                let v = frame.pop_long()?;
                frame.push(Slot::Double(v as f64))?;
            }
            Instruction::F2i => {
                let v = frame.pop_float()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::F2l => {
                let v = frame.pop_float()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::F2d => {
                let v = frame.pop_float()?;
                frame.push(Slot::Double(f64::from(v)))?;
            }
            Instruction::D2i => {
                let v = frame.pop_double()?;
                frame.push(Slot::Int(v as i32))?;
            }
            Instruction::D2l => {
                let v = frame.pop_double()?;
                frame.push(Slot::Long(v as i64))?;
            }
            Instruction::D2f => {
                let v = frame.pop_double()?;
                frame.push(Slot::Float(v as f32))?;
            }
            Instruction::I2b => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as i8)))?;
            }
            Instruction::I2c => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as u16)))?;
            }
            Instruction::I2s => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(i32::from(v as i16)))?;
            }
            Instruction::Goto(offset) => jump!(*offset),
            Instruction::GotoW(offset) => jump!(*offset),
            Instruction::Ifeq(offset) => {
                if frame.pop_int()? == 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifne(offset) => {
                if frame.pop_int()? != 0 {
                    jump!(*offset);
                }
            }
            Instruction::Iflt(offset) => {
                if frame.pop_int()? < 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifge(offset) => {
                if frame.pop_int()? >= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifgt(offset) => {
                if frame.pop_int()? > 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifle(offset) => {
                if frame.pop_int()? <= 0 {
                    jump!(*offset);
                }
            }
            Instruction::Ifnull(offset) => {
                if matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::Ifnonnull(offset) => {
                if !matches!(frame.pop()?, Slot::Reference(None)) {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpeq(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpne(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a != b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmplt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a < b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpge(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a >= b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmpgt(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a > b {
                    jump!(*offset);
                }
            }
            Instruction::IfIcmple(offset) => {
                let b = frame.pop_int()?;
                let a = frame.pop_int()?;
                if a <= b {
                    jump!(*offset);
                }
            }
            Instruction::IfAcmpeq(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a == b {
                    jump!(*offset);
                }
            }
            Instruction::IfAcmpne(offset) => {
                let b = frame.pop()?;
                let a = frame.pop()?;
                if a != b {
                    jump!(*offset);
                }
            }

            // ---- Object allocation ----
            Instruction::New(cp_idx) => {
                let target_class = {
                    let ctx = registry.get(&current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                registry.ensure_loaded(&target_class, loader)?;
                ensure_initialized(
                    registry,
                    loader,
                    heap,
                    stdout,
                    &target_class,
                    &current_class,
                )?;
                // Walk the super chain to sum all instance field counts
                // (e.g. Enum has 2 fields inherited by every enum subclass).
                let field_count = total_instance_field_count(registry, &target_class);
                #[cfg(feature = "telemetry")]
                registry.telemetry.object_lineage.record(
                    &current_class,
                    &current_method,
                    pc,
                    &target_class,
                );
                let r = heap.allocate(target_class.clone(), field_count);
                // Set reference/long/float/double fields to their JVM-spec defaults.
                // heap.allocate initialises everything to Int(0), which is wrong
                // for reference-typed fields (should be Reference(None)).
                init_object_fields(registry, heap, r, &target_class);
                frame.push(Slot::Reference(Some(r)))?;
                if heap.should_gc() {
                    let roots = gather_roots(&frame, &call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(&mut frame, &mut call_stack, registry, heap);
                }
            }

            // ---- Field access ----
            Instruction::Getfield(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let r = frame.pop_ref()?;
                registry.ensure_loaded(&target_class, loader)?;
                let fidx = field_slot_idx(registry, &target_class, &field_name)?;
                let val = heap.get(r)?.fields[fidx];
                frame.push(val)?;
            }
            Instruction::Putfield(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let val = frame.pop()?;
                let r = frame.pop_ref()?;
                registry.ensure_loaded(&target_class, loader)?;
                let fidx = field_slot_idx(registry, &target_class, &field_name)?;
                heap.write_field(r, fidx, val)?;
            }
            Instruction::Getstatic(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                registry.ensure_loaded(&target_class, loader)?;
                ensure_initialized(
                    registry,
                    loader,
                    heap,
                    stdout,
                    &target_class,
                    &current_class,
                )?;
                let sidx = static_field_idx(registry.get(&target_class)?, &field_name)?;
                let val = registry.get(&target_class)?.static_fields[sidx];
                frame.push(val)?;
            }
            Instruction::Putstatic(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let val = frame.pop()?;
                registry.ensure_loaded(&target_class, loader)?;
                ensure_initialized(
                    registry,
                    loader,
                    heap,
                    stdout,
                    &target_class,
                    &current_class,
                )?;
                let sidx = static_field_idx(registry.get(&target_class)?, &field_name)?;
                registry.get_mut(&target_class)?.static_fields[sidx] = val;
            }

            // ---- Instance method dispatch ----
            //
            // invokespecial and invokevirtual: resolve class+name+descriptor from
            // the Methodref.  Dispatch cross-class via registry; unloadable
            // classes (e.g. java/lang/Object) fall back to no-op.
            Instruction::Invokespecial(cp_idx) | Instruction::Invokevirtual(cp_idx) => {
                // Fast path: cache hit for invokespecial (static dispatch — safe to cache).
                if matches!(instr, Instruction::Invokespecial(_))
                    && let Some(&(ref cached_cls, cached_idx, cached_ac)) = dispatch_cache
                        .get(current_class.as_str())
                        .and_then(|m| m.get(&cp_idx.0))
                {
                    let (callee_pc_to_idx, callee_frame) = {
                        let ctx = registry.get(cached_cls)?;
                        let max_locals = usize::from(ctx.methods[cached_idx].max_locals);
                        let max_stack = usize::from(ctx.methods[cached_idx].max_stack);
                        let pci = std::sync::Arc::clone(&ctx.methods[cached_idx].pc_to_idx);
                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                        locals_buf.resize(max_locals, Slot::Int(0));
                        if cached_ac + 1 > max_locals {
                            return Err(VmError::LocalOutOfBounds {
                                index: cached_ac + 1,
                                max_locals,
                            });
                        }
                        for i in (1..=cached_ac).rev() {
                            locals_buf[i] = frame.pop()?;
                        }
                        locals_buf[0] = frame.pop()?; // `this`
                        let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                        (pci, f)
                    };
                    let dispatch_class = cached_cls.clone();
                    let callee_idx = cached_idx;
                    call_stack.push(CallFrame {
                        frame,
                        method_idx,
                        pc_to_idx,
                        resume_idx: idx + 1,
                        class_name: current_class.clone(),
                    });
                    frame = callee_frame;
                    method_idx = callee_idx;
                    pc_to_idx = callee_pc_to_idx;
                    current_class = dispatch_class;
                    instructions = std::sync::Arc::clone(
                        &registry.get(&current_class)?.methods[method_idx].instructions,
                    );
                    #[cfg(feature = "telemetry")]
                    {
                        current_method = registry
                            .get(&current_class)
                            .map(|c| {
                                c.methods
                                    .get(method_idx)
                                    .map(|m| m.name.clone())
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default();
                    }
                    idx = 0;
                    continue;
                }
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                // <clinit> (static initialiser) is not supported yet — skip silently.
                if callee_name == "<clinit>" {
                    idx += 1;
                    continue;
                }
                // Attempt to load the target class; soft-fail for unloadable.
                let class_was_loaded = registry.ensure_loaded(&callee_class, loader)?;
                let resolved = if class_was_loaded {
                    resolve_method_in_hierarchy(
                        registry,
                        loader,
                        &callee_class,
                        &callee_name,
                        &callee_desc,
                    )
                } else {
                    None
                };
                let (dispatch_class, callee_idx) = match resolved {
                    Some((cls, i)) => (cls, i),
                    None => {
                        // Check lambda dispatch before native fallback.
                        let arg_count = parse_arg_count(&callee_desc);
                        let stack_len = frame.stack_len();
                        if stack_len > arg_count {
                            let this_pos = stack_len - arg_count - 1;
                            let actual_class_opt =
                                if let Ok(Slot::Reference(Some(r))) = frame.peek_at(this_pos) {
                                    heap.get(r).ok().map(|o| o.class_name.clone())
                                } else {
                                    None
                                };

                            if let Some(ref actual_class) = actual_class_opt
                                && let Some(lambda_info) =
                                    registry.get_lambda(actual_class).cloned()
                                && callee_name == lambda_info.sam_method
                            {
                                let mut sam_args = vec![Slot::Int(0); arg_count];
                                for i in (0..arg_count).rev() {
                                    sam_args[i] = frame.pop()?;
                                }
                                let this_slot = frame.pop()?;
                                let this_ref = match &this_slot {
                                    Slot::Reference(Some(r)) => *r,
                                    _ => return Err(VmError::NullPointerException),
                                };

                                let obj = heap.get(this_ref)?;
                                let mut impl_args: Vec<Slot> = Vec::new();
                                for i in 0..lambda_info.captured_count {
                                    impl_args.push(obj.fields[i]);
                                }
                                impl_args.extend(sam_args.iter().copied());

                                let _ = registry.ensure_loaded(&lambda_info.impl_class, loader);

                                let resolved = resolve_method_in_hierarchy(
                                    registry,
                                    loader,
                                    &lambda_info.impl_class,
                                    &lambda_info.impl_method,
                                    &lambda_info.impl_desc,
                                );
                                if let Some((dispatch_class, impl_idx)) = resolved {
                                    let (callee_pc_to_idx, callee_frame) = {
                                        let ctx = registry.get(&dispatch_class)?;
                                        let max_locals =
                                            usize::from(ctx.methods[impl_idx].max_locals);
                                        let max_stack =
                                            usize::from(ctx.methods[impl_idx].max_stack);
                                        let pci =
                                            std::sync::Arc::clone(&ctx.methods[impl_idx].pc_to_idx);
                                        let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                        locals_buf.resize(max_locals, Slot::Int(0));
                                        for (i, slot) in impl_args.into_iter().enumerate() {
                                            locals_buf[i] = slot;
                                        }
                                        let f =
                                            Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                                        (pci, f)
                                    };
                                    call_stack.push(CallFrame {
                                        frame,
                                        method_idx,
                                        pc_to_idx,
                                        resume_idx: idx + 1,
                                        class_name: current_class.clone(),
                                    });
                                    frame = callee_frame;
                                    method_idx = impl_idx;
                                    pc_to_idx = callee_pc_to_idx;
                                    current_class = dispatch_class;
                                    instructions = std::sync::Arc::clone(
                                        &registry.get(&current_class)?.methods[method_idx]
                                            .instructions,
                                    );
                                    #[cfg(feature = "telemetry")]
                                    {
                                        current_method = registry
                                            .get(&current_class)
                                            .map(|c| {
                                                c.methods
                                                    .get(method_idx)
                                                    .map(|m| m.name.clone())
                                                    .unwrap_or_default()
                                            })
                                            .unwrap_or_default();
                                    }
                                    idx = 0;
                                    continue;
                                }
                            }
                        }
                        // Check native registry, walking the super chain.
                        let native_handler_kind = {
                            let mut found = registry.natives_mut().get_kind(
                                &callee_class,
                                &callee_name,
                                &callee_desc,
                            );
                            if found.is_none() {
                                // Walk super chain for native lookup (e.g. Enum.ordinal
                                // called via SimpleEnum$Color.ordinal).
                                let start = if callee_class.starts_with('[') {
                                    // Arrays inherit from Object.
                                    Some("java/lang/Object".to_string())
                                } else {
                                    registry
                                        .get(&callee_class)
                                        .ok()
                                        .and_then(|c| c.super_class.clone())
                                };
                                let mut sc = start;
                                while let Some(ref s) = sc {
                                    if let Some(h) = registry.natives_mut().get_kind(
                                        s,
                                        &callee_name,
                                        &callee_desc,
                                    ) {
                                        found = Some(h);
                                        break;
                                    }
                                    sc = registry.get(s).ok().and_then(|c| c.super_class.clone());
                                }
                            }
                            found
                        };
                        match native_handler_kind {
                            Some(HandlerKind::Simple(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                let mut native_args = vec![Slot::Int(0); arg_count];
                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }
                                let this_slot = frame.pop()?; // pop `this`
                                native_args.insert(0, this_slot);
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let result = handler(&native_args, heap, stdout);
                                #[cfg(feature = "telemetry")]
                                {
                                    registry.telemetry.native_boundary.record_call(
                                        &callee_class,
                                        &callee_name,
                                        _native_start.elapsed().as_nanos() as u64,
                                        result.is_err(),
                                    );
                                    if matches!(instr, Instruction::Invokevirtual(_)) {
                                        // Native methods do not perform a bytecode hierarchy
                                        // walk — hierarchy_walk is always false here.
                                        registry.telemetry.dispatch_resolution.record(
                                            &current_class,
                                            cp_idx.0,
                                            &callee_class,
                                            false,
                                        );
                                    }
                                }
                                let result = match result {
                                    Ok(result) => result,
                                    Err(VmError::JavaException { class_name }) => {
                                        let exception_ref = materialize_java_exception_object(
                                            registry,
                                            loader,
                                            heap,
                                            &class_name,
                                        )?;
                                        propagate_java_exception!(class_name, exception_ref, pc);
                                    }
                                    Err(err) => return Err(err),
                                };
                                if let Some(val) = result {
                                    frame.push(val)?;
                                }
                                idx += 1;
                                continue;
                            }
                            Some(HandlerKind::Callback(handler)) => {
                                let arg_count = parse_arg_count(&callee_desc);
                                let mut native_args = vec![Slot::Int(0); arg_count];
                                for i in (0..arg_count).rev() {
                                    native_args[i] = frame.pop()?;
                                }
                                let this_slot = frame.pop()?; // pop `this`
                                native_args.insert(0, this_slot);
                                #[cfg(feature = "telemetry")]
                                let _native_start = std::time::Instant::now();
                                let mut invoke_cb = create_invoke_cb!(registry, loader);
                                let result = handler(&native_args, heap, stdout, &mut invoke_cb);
                                #[cfg(feature = "telemetry")]
                                {
                                    registry.telemetry.native_boundary.record_call(
                                        &callee_class,
                                        &callee_name,
                                        _native_start.elapsed().as_nanos() as u64,
                                        result.is_err(),
                                    );
                                    if matches!(instr, Instruction::Invokevirtual(_)) {
                                        registry.telemetry.dispatch_resolution.record(
                                            &current_class,
                                            cp_idx.0,
                                            &callee_class,
                                            false,
                                        );
                                    }
                                }
                                let result = match result {
                                    Ok(result) => result,
                                    Err(VmError::JavaException { class_name }) => {
                                        let exception_ref = materialize_java_exception_object(
                                            registry,
                                            loader,
                                            heap,
                                            &class_name,
                                        )?;
                                        propagate_java_exception!(class_name, exception_ref, pc);
                                    }
                                    Err(err) => return Err(err),
                                };
                                if let Some(val) = result {
                                    frame.push(val)?;
                                }
                                idx += 1;
                                continue;
                            }
                            None => {
                                // Unloadable or missing — pop args + this and continue.
                                let arg_count = parse_arg_count(&callee_desc);
                                for _ in 0..arg_count {
                                    frame.pop()?;
                                }
                                frame.pop()?; // pop `this`
                                idx += 1;
                                continue;
                            }
                        }
                    }
                };
                let arg_count = parse_arg_count(&callee_desc);
                // Populate dispatch cache for invokespecial (static dispatch — result is stable).
                if matches!(instr, Instruction::Invokespecial(_)) {
                    dispatch_cache
                        .entry(current_class.clone())
                        .or_default()
                        .insert(cp_idx.0, (dispatch_class.clone(), callee_idx, arg_count));
                }
                #[cfg(feature = "telemetry")]
                if matches!(instr, Instruction::Invokevirtual(_)) {
                    registry.telemetry.dispatch_resolution.record(
                        &current_class,
                        cp_idx.0,
                        &dispatch_class,
                        dispatch_class != callee_class,
                    );
                }
                let (callee_pc_to_idx, callee_frame) = {
                    let ctx = registry.get(&dispatch_class)?;
                    let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                    let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                    let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                    let (mut locals_buf, stack_buf) = frame_pool.acquire();
                    locals_buf.resize(max_locals, Slot::Int(0));
                    if arg_count + 1 > max_locals {
                        return Err(VmError::LocalOutOfBounds {
                            index: arg_count + 1,
                            max_locals,
                        });
                    }
                    // Pop args into locals[1..=arg_count] in reverse (stack top = last arg).
                    for i in (1..=arg_count).rev() {
                        locals_buf[i] = frame.pop()?;
                    }
                    locals_buf[0] = frame.pop()?; // `this`
                    let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                    (pci, f)
                };
                call_stack.push(CallFrame {
                    frame,
                    method_idx,
                    pc_to_idx,
                    resume_idx: idx + 1,
                    class_name: current_class.clone(),
                });
                frame = callee_frame;
                method_idx = callee_idx;
                pc_to_idx = callee_pc_to_idx;
                current_class = dispatch_class;
                instructions = std::sync::Arc::clone(
                    &registry.get(&current_class)?.methods[method_idx].instructions,
                );
                #[cfg(feature = "telemetry")]
                {
                    current_method = registry
                        .get(&current_class)
                        .map(|c| {
                            c.methods
                                .get(method_idx)
                                .map(|m| m.name.clone())
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();
                }
                idx = 0;
                continue;
            }

            // ---- Reference return ----
            Instruction::Areturn => {
                let v = frame.pop()?;
                do_return!(Some(v));
            }

            // ----------------------------------------------------------------
            // Array allocation
            // ----------------------------------------------------------------
            Instruction::Newarray(array_type) => {
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let class_name = match array_type {
                    ArrayType::Boolean => "[Z",
                    ArrayType::Char => "[C",
                    ArrayType::Float => "[F",
                    ArrayType::Double => "[D",
                    ArrayType::Byte => "[B",
                    ArrayType::Short => "[S",
                    ArrayType::Int => "[I",
                    ArrayType::Long => "[J",
                };
                let r = heap.allocate(class_name.to_string(), count as usize);
                // Fix element types for non-int primitive arrays.
                match array_type {
                    ArrayType::Long => {
                        let obj = heap.get_mut(r)?;
                        for slot in &mut obj.fields {
                            *slot = Slot::Long(0);
                        }
                    }
                    ArrayType::Float => {
                        let obj = heap.get_mut(r)?;
                        for slot in &mut obj.fields {
                            *slot = Slot::Float(0.0);
                        }
                    }
                    ArrayType::Double => {
                        let obj = heap.get_mut(r)?;
                        for slot in &mut obj.fields {
                            *slot = Slot::Double(0.0);
                        }
                    }
                    _ => {} // Int/Boolean/Byte/Char/Short default to Slot::Int(0)
                }
                frame.push(Slot::Reference(Some(r)))?;
                if heap.should_gc() {
                    let roots = gather_roots(&frame, &call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(&mut frame, &mut call_stack, registry, heap);
                }
            }
            Instruction::Anewarray(cp_idx) => {
                let element_type = {
                    let ctx = registry.get(&current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let array_type = format!("[L{element_type};");
                let count = frame.pop_int()?;
                if count < 0 {
                    return Err(VmError::NegativeArraySize { size: count });
                }
                let r = heap.allocate(array_type, count as usize);
                // Fix elements to Reference(None).
                let obj = heap.get_mut(r)?;
                for slot in &mut obj.fields {
                    *slot = Slot::Reference(None);
                }
                frame.push(Slot::Reference(Some(r)))?;
                if heap.should_gc() {
                    let roots = gather_roots(&frame, &call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(&mut frame, &mut call_stack, registry, heap);
                }
            }
            Instruction::Arraylength => {
                let r = frame.pop_ref()?;
                let len = heap.get(r)?.fields.len();
                frame.push(Slot::Int(len as i32))?;
            }

            // ---- Int array ----
            Instruction::Iaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_int()?
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Iastore => {
                let val = frame.pop_int()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Long array ----
            Instruction::Laload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_long()?
                };
                frame.push(Slot::Long(v))?;
            }
            Instruction::Lastore => {
                let val = frame.pop_long()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Long(val))?;
            }

            // ---- Float array ----
            Instruction::Faload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_float()?
                };
                frame.push(Slot::Float(v))?;
            }
            Instruction::Fastore => {
                let val = frame.pop_float()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Float(val))?;
            }

            // ---- Double array ----
            Instruction::Daload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize].as_double()?
                };
                frame.push(Slot::Double(v))?;
            }
            Instruction::Dastore => {
                let val = frame.pop_double()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Double(val))?;
            }

            // ---- Reference array ----
            Instruction::Aaload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    fields[idx_val as usize]
                };
                frame.push(v)?;
            }
            Instruction::Aastore => {
                let val = frame.pop()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, val)?;
            }

            // ---- Byte/boolean array (stored as Int, truncated to i8) ----
            Instruction::Baload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    i32::from(fields[idx_val as usize].as_int()? as i8)
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Bastore => {
                let val = i32::from(frame.pop_int()? as i8);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Char array (stored as Int, masked to u16) ----
            Instruction::Caload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    i32::from(fields[idx_val as usize].as_int()? as u16)
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Castore => {
                let val = i32::from(frame.pop_int()? as u16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Short array (stored as Int, truncated to i16) ----
            Instruction::Saload => {
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let v = {
                    let fields = &heap.get(r)?.fields;
                    if idx_val < 0 || idx_val as usize >= fields.len() {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: fields.len(),
                        });
                    }
                    i32::from(fields[idx_val as usize].as_int()? as i16)
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Sastore => {
                let val = i32::from(frame.pop_int()? as i16);
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                {
                    let len = heap.get(r)?.fields.len();
                    if idx_val < 0 || idx_val as usize >= len {
                        return Err(VmError::ArrayIndexOutOfBounds {
                            index: idx_val,
                            length: len,
                        });
                    }
                }
                heap.write_field(r, idx_val as usize, Slot::Int(val))?;
            }

            // ---- Switch ----
            Instruction::Tableswitch {
                default,
                low,
                high,
                offsets,
            } => {
                let key = frame.pop_int()?;
                let offset = if key >= *low && key <= *high {
                    offsets[(key - low) as usize]
                } else {
                    *default
                };
                jump!(offset);
            }
            Instruction::Lookupswitch { default, pairs } => {
                let key = frame.pop_int()?;
                let offset = pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map_or(*default, |(_, off)| *off);
                jump!(offset);
            }

            // ---- checkcast / instanceof ----
            Instruction::Checkcast(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(slot)?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = {
                            let ctx = registry.get(&current_class)?;
                            resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                        };
                        let actual = heap.get(*r)?.class_name.clone();
                        if is_assignable_from(registry, loader, &actual, &target) {
                            frame.push(slot)?;
                        } else {
                            return Err(VmError::ClassCastException {
                                from: actual,
                                to: target,
                            });
                        }
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }
            Instruction::Instanceof(cp_idx) => {
                let slot = frame.pop()?;
                match &slot {
                    Slot::Reference(None) => {
                        frame.push(Slot::Int(0))?;
                    }
                    Slot::Reference(Some(r)) => {
                        let target = {
                            let ctx = registry.get(&current_class)?;
                            resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                        };
                        let actual = heap.get(*r)?.class_name.clone();
                        let result =
                            i32::from(is_assignable_from(registry, loader, &actual, &target));
                        frame.push(Slot::Int(result))?;
                    }
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "reference",
                            got: "non-reference",
                        });
                    }
                }
            }

            // ---- athrow with exception table dispatch ----
            Instruction::Athrow => {
                let exception_ref = frame.pop_ref()?;
                let exc_class_name = heap.get(exception_ref)?.class_name.clone();
                propagate_java_exception!(exc_class_name, exception_ref, pc);
            }

            // ----------------------------------------------------------------
            // invokedynamic — resolve bootstrap method, dispatch based on
            // the bootstrap class (StringConcatFactory, LambdaMetafactory).
            // ----------------------------------------------------------------
            Instruction::Invokedynamic(cp_idx) => {
                let cp_idx_val = usize::from(cp_idx.0);

                // 1. Resolve InvokeDynamic CP entry.
                let (bsm_idx, call_name, call_desc) = {
                    let ctx = registry.get(&current_class)?;
                    let cp = &ctx.constant_pool;
                    match cp.get(cp_idx_val).and_then(|e| e.as_ref()) {
                        Some(CpEntry::InvokeDynamic {
                            bootstrap_method_attr_index,
                            name_and_type_index,
                        }) => {
                            let (name, desc) =
                                resolve_name_and_type(cp, name_and_type_index.0 as usize)?;
                            (*bootstrap_method_attr_index as usize, name, desc)
                        }
                        _ => return Err(VmError::InvalidCpIndex { index: cp_idx_val }),
                    }
                };

                // 2. Look up the bootstrap method entry.
                let (bsm_class, bsm_args) = {
                    let ctx = registry.get(&current_class)?;
                    let bsm_entry = ctx
                        .bootstrap_methods
                        .get(bsm_idx)
                        .ok_or(VmError::InvalidCpIndex { index: bsm_idx })?;
                    let (_kind, class, _name, _desc) =
                        resolve_method_handle(&ctx.constant_pool, bsm_entry.method_ref.0 as usize)?;
                    let args: Vec<duke_classfile::types::CpIndex> = bsm_entry.arguments.clone();
                    (class, args)
                };

                let _ = &call_name; // suppress unused warning for now

                // 3. Dispatch based on bootstrap method class.
                if bsm_class == "java/lang/invoke/StringConcatFactory" {
                    // --- StringConcatFactory.makeConcatWithConstants ---
                    let arg_count = parse_arg_count(&call_desc);
                    let arg_types = parse_arg_types(&call_desc);
                    let mut dynamic_args = vec![Slot::Int(0); arg_count];
                    for i in (0..arg_count).rev() {
                        dynamic_args[i] = frame.pop()?;
                    }

                    // Resolve recipe (first bootstrap arg) and constants (remaining).
                    let (recipe, constants) = {
                        let ctx = registry.get(&current_class)?;
                        let cp = &ctx.constant_pool;
                        let recipe = if bsm_args.is_empty() {
                            String::new()
                        } else {
                            resolve_cp_string(cp, bsm_args[0].0 as usize)?
                        };
                        let mut consts = Vec::new();
                        for arg_idx in bsm_args.iter().skip(1) {
                            if let Ok(s) = resolve_cp_string(cp, arg_idx.0 as usize) {
                                consts.push(s);
                            }
                        }
                        (recipe, consts)
                    };

                    let result = execute_string_concat_recipe(
                        &recipe,
                        &dynamic_args,
                        &arg_types,
                        &constants,
                        heap,
                    )?;
                    frame.push(result)?;
                } else if bsm_class == "java/lang/invoke/LambdaMetafactory" {
                    // --- LambdaMetafactory.metafactory ---
                    // Bootstrap args: [MethodType erased, MethodHandle impl, MethodType specialized]

                    let (impl_kind, impl_class, impl_method, impl_desc) = {
                        let ctx = registry.get(&current_class)?;
                        let cp = &ctx.constant_pool;
                        if bsm_args.len() < 3 {
                            return Err(VmError::Unimplemented {
                                mnemonic: "LambdaMetafactory requires 3 bootstrap args",
                            });
                        }
                        resolve_method_handle(cp, bsm_args[1].0 as usize)?
                    };

                    let sam_method = call_name.clone();

                    // Resolve erased SAM descriptor from bootstrap arg 0.
                    let sam_desc = {
                        let ctx = registry.get(&current_class)?;
                        let cp = &ctx.constant_pool;
                        match cp.get(bsm_args[0].0 as usize).and_then(|e| e.as_ref()) {
                            Some(CpEntry::MethodType { descriptor_index }) => {
                                match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                                    Some(CpEntry::Utf8(s)) => s.clone(),
                                    _ => {
                                        return Err(VmError::InvalidCpIndex {
                                            index: descriptor_index.0 as usize,
                                        });
                                    }
                                }
                            }
                            _ => {
                                return Err(VmError::InvalidCpIndex {
                                    index: bsm_args[0].0 as usize,
                                });
                            }
                        }
                    };

                    // Pop captured variables from the stack.
                    let captured_count = parse_arg_count(&call_desc);
                    let mut captured_args: Vec<Slot> = (0..captured_count)
                        .map(|_| frame.pop())
                        .collect::<VmResult<Vec<_>>>()?;
                    captured_args.reverse();

                    let lambda_info = LambdaInfo {
                        impl_class: impl_class.clone(),
                        impl_method: impl_method.clone(),
                        impl_desc: impl_desc.clone(),
                        impl_kind,
                        sam_method,
                        sam_desc,
                        captured_count,
                    };
                    let lambda_class = registry.register_lambda(lambda_info);

                    let r = heap.allocate(lambda_class, captured_count);
                    for (i, slot) in captured_args.into_iter().enumerate() {
                        heap.get_mut(r)?.fields[i] = slot;
                    }

                    let _ = registry.ensure_loaded(&impl_class, loader);

                    frame.push(Slot::Reference(Some(r)))?;
                    if heap.should_gc() {
                        let roots = gather_roots(&frame, &call_stack, registry);
                        heap.collect(&roots);
                        patch_forwarded_slots(&mut frame, &mut call_stack, registry, heap);
                    }
                } else {
                    // Unknown bootstrap method — pop args and push null.
                    let arg_count = parse_arg_count(&call_desc);
                    for _ in 0..arg_count {
                        frame.pop()?;
                    }
                    if !call_desc.ends_with(")V") {
                        frame.push(Slot::Reference(None))?;
                    }
                }
            }

            // ----------------------------------------------------------------
            // invokeinterface — like invokevirtual but resolves from
            // InterfaceMethodref and dispatches on the actual object class.
            // ----------------------------------------------------------------
            Instruction::Invokeinterface {
                index: cp_idx,
                count: _,
            } => {
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                if callee_name == "<clinit>" {
                    idx += 1;
                    continue;
                }
                let arg_count = parse_arg_count(&callee_desc);
                // Peek at `this` (sits below the args) to determine the actual
                // runtime class without consuming the stack yet.  Each dispatch
                // path (native / lambda / bytecode) pops what it needs itself.
                let stack_len = frame.stack_len();
                let actual_class = if stack_len > arg_count {
                    let this_pos = stack_len - arg_count - 1;
                    if let Ok(Slot::Reference(Some(r))) = frame.peek_at(this_pos) {
                        heap.get(r).ok().map(|o| o.class_name.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
                .unwrap_or_else(|| callee_class.clone());

                // Try to find the method on the actual class (walking hierarchy).
                let resolved = resolve_method_in_hierarchy(
                    registry,
                    loader,
                    &actual_class,
                    &callee_name,
                    &callee_desc,
                );

                let (dispatch_class, callee_idx) = if let Some(pair) = resolved {
                    pair
                } else {
                    // Fall back to interface class hierarchy.
                    let iface_resolved = resolve_method_in_hierarchy(
                        registry,
                        loader,
                        &callee_class,
                        &callee_name,
                        &callee_desc,
                    );
                    match iface_resolved {
                        Some(pair) => pair,
                        None => {
                            // Check native registry — try actual class then interface class.
                            let native_kind = registry
                                .natives_mut()
                                .get_kind(&actual_class, &callee_name, &callee_desc)
                                .or_else(|| {
                                    registry.natives_mut().get_kind(
                                        &callee_class,
                                        &callee_name,
                                        &callee_desc,
                                    )
                                });
                            match native_kind {
                                Some(HandlerKind::Simple(handler)) => {
                                    // Native path: collect args + this into a Vec<Slot>
                                    // for the handler(&[Slot], ...) signature.
                                    let mut callee_args = vec![Slot::Int(0); arg_count];
                                    for i in (0..arg_count).rev() {
                                        callee_args[i] = frame.pop()?;
                                    }
                                    let this_slot = frame.pop()?;
                                    callee_args.insert(0, this_slot);
                                    #[cfg(feature = "telemetry")]
                                    let _native_start = std::time::Instant::now();
                                    let result = handler(&callee_args, heap, stdout);
                                    #[cfg(feature = "telemetry")]
                                    {
                                        registry.telemetry.native_boundary.record_call(
                                            &callee_class,
                                            &callee_name,
                                            _native_start.elapsed().as_nanos() as u64,
                                            result.is_err(),
                                        );
                                        // Native interface methods skip the bytecode
                                        // hierarchy walk — hierarchy_walk is always false here.
                                        registry.telemetry.dispatch_resolution.record(
                                            &current_class,
                                            cp_idx.0,
                                            &actual_class,
                                            false,
                                        );
                                    }
                                    let result = match result {
                                        Ok(result) => result,
                                        Err(VmError::JavaException { class_name }) => {
                                            let exception_ref = materialize_java_exception_object(
                                                registry,
                                                loader,
                                                heap,
                                                &class_name,
                                            )?;
                                            propagate_java_exception!(
                                                class_name,
                                                exception_ref,
                                                pc
                                            );
                                        }
                                        Err(err) => return Err(err),
                                    };
                                    if let Some(val) = result {
                                        frame.push(val)?;
                                    }
                                    idx += 1;
                                    continue;
                                }
                                Some(HandlerKind::Callback(handler)) => {
                                    let mut callee_args = vec![Slot::Int(0); arg_count];
                                    for i in (0..arg_count).rev() {
                                        callee_args[i] = frame.pop()?;
                                    }
                                    let this_slot = frame.pop()?;
                                    callee_args.insert(0, this_slot);
                                    #[cfg(feature = "telemetry")]
                                    let _native_start = std::time::Instant::now();
                                    let mut invoke_cb = create_invoke_cb!(registry, loader);
                                    let result =
                                        handler(&callee_args, heap, stdout, &mut invoke_cb);
                                    #[cfg(feature = "telemetry")]
                                    {
                                        registry.telemetry.native_boundary.record_call(
                                            &callee_class,
                                            &callee_name,
                                            _native_start.elapsed().as_nanos() as u64,
                                            result.is_err(),
                                        );
                                        registry.telemetry.dispatch_resolution.record(
                                            &current_class,
                                            cp_idx.0,
                                            &actual_class,
                                            false,
                                        );
                                    }
                                    let result = match result {
                                        Ok(result) => result,
                                        Err(VmError::JavaException { class_name }) => {
                                            let exception_ref = materialize_java_exception_object(
                                                registry,
                                                loader,
                                                heap,
                                                &class_name,
                                            )?;
                                            propagate_java_exception!(
                                                class_name,
                                                exception_ref,
                                                pc
                                            );
                                        }
                                        Err(err) => return Err(err),
                                    };
                                    if let Some(val) = result {
                                        frame.push(val)?;
                                    }
                                    idx += 1;
                                    continue;
                                }
                                None => {} // fall through to lambda / no-op
                            }
                            // Check lambda registry for SAM dispatch.
                            if let Some(lambda_info) = registry.get_lambda(&actual_class).cloned()
                                && callee_name == lambda_info.sam_method
                            {
                                // Lambda path: collect args + this into a Vec<Slot>.
                                let mut callee_args = vec![Slot::Int(0); arg_count];
                                for i in (0..arg_count).rev() {
                                    callee_args[i] = frame.pop()?;
                                }
                                let this_slot = frame.pop()?;
                                callee_args.insert(0, this_slot);
                                let this_ref = match &callee_args[0] {
                                    Slot::Reference(Some(r)) => *r,
                                    _ => return Err(VmError::NullPointerException),
                                };
                                let obj = heap.get(this_ref)?;
                                let mut impl_args: Vec<Slot> = Vec::new();
                                for i in 0..lambda_info.captured_count {
                                    impl_args.push(obj.fields[i]);
                                }
                                impl_args.extend(callee_args[1..].iter().copied());

                                let _ = registry.ensure_loaded(&lambda_info.impl_class, loader);

                                if lambda_info.impl_kind == 6 {
                                    // invokeStatic dispatch
                                    let resolved = resolve_method_in_hierarchy(
                                        registry,
                                        loader,
                                        &lambda_info.impl_class,
                                        &lambda_info.impl_method,
                                        &lambda_info.impl_desc,
                                    );
                                    if let Some((dispatch_class, impl_idx)) = resolved {
                                        let (callee_pc_to_idx, callee_frame) = {
                                            let ctx = registry.get(&dispatch_class)?;
                                            let max_locals =
                                                usize::from(ctx.methods[impl_idx].max_locals);
                                            let max_stack =
                                                usize::from(ctx.methods[impl_idx].max_stack);
                                            let pci = std::sync::Arc::clone(
                                                &ctx.methods[impl_idx].pc_to_idx,
                                            );
                                            let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                            locals_buf.resize(max_locals, Slot::Int(0));
                                            for (i, slot) in impl_args.into_iter().enumerate() {
                                                locals_buf[i] = slot;
                                            }
                                            let f = Frame::from_pool_bufs(
                                                locals_buf, stack_buf, max_stack,
                                            );
                                            (pci, f)
                                        };
                                        call_stack.push(CallFrame {
                                            frame,
                                            method_idx,
                                            pc_to_idx,
                                            resume_idx: idx + 1,
                                            class_name: current_class.clone(),
                                        });
                                        frame = callee_frame;
                                        method_idx = impl_idx;
                                        pc_to_idx = callee_pc_to_idx;
                                        current_class = dispatch_class;
                                        instructions = std::sync::Arc::clone(
                                            &registry.get(&current_class)?.methods[method_idx]
                                                .instructions,
                                        );
                                        #[cfg(feature = "telemetry")]
                                        {
                                            current_method = registry
                                                .get(&current_class)
                                                .map(|c| {
                                                    c.methods
                                                        .get(method_idx)
                                                        .map(|m| m.name.clone())
                                                        .unwrap_or_default()
                                                })
                                                .unwrap_or_default();
                                        }
                                        idx = 0;
                                        continue;
                                    }
                                } else if lambda_info.impl_kind == 5 || lambda_info.impl_kind == 9 {
                                    // invokeVirtual / invokeInterface dispatch
                                    let resolved = resolve_method_in_hierarchy(
                                        registry,
                                        loader,
                                        &lambda_info.impl_class,
                                        &lambda_info.impl_method,
                                        &lambda_info.impl_desc,
                                    );
                                    if let Some((dispatch_class, impl_idx)) = resolved {
                                        let (callee_pc_to_idx, callee_frame) = {
                                            let ctx = registry.get(&dispatch_class)?;
                                            let max_locals =
                                                usize::from(ctx.methods[impl_idx].max_locals);
                                            let max_stack =
                                                usize::from(ctx.methods[impl_idx].max_stack);
                                            let pci = std::sync::Arc::clone(
                                                &ctx.methods[impl_idx].pc_to_idx,
                                            );
                                            let (mut locals_buf, stack_buf) = frame_pool.acquire();
                                            locals_buf.resize(max_locals, Slot::Int(0));
                                            for (i, slot) in impl_args.into_iter().enumerate() {
                                                locals_buf[i] = slot;
                                            }
                                            let f = Frame::from_pool_bufs(
                                                locals_buf, stack_buf, max_stack,
                                            );
                                            (pci, f)
                                        };
                                        call_stack.push(CallFrame {
                                            frame,
                                            method_idx,
                                            pc_to_idx,
                                            resume_idx: idx + 1,
                                            class_name: current_class.clone(),
                                        });
                                        frame = callee_frame;
                                        method_idx = impl_idx;
                                        pc_to_idx = callee_pc_to_idx;
                                        current_class = dispatch_class;
                                        instructions = std::sync::Arc::clone(
                                            &registry.get(&current_class)?.methods[method_idx]
                                                .instructions,
                                        );
                                        #[cfg(feature = "telemetry")]
                                        {
                                            current_method = registry
                                                .get(&current_class)
                                                .map(|c| {
                                                    c.methods
                                                        .get(method_idx)
                                                        .map(|m| m.name.clone())
                                                        .unwrap_or_default()
                                                })
                                                .unwrap_or_default();
                                        }
                                        idx = 0;
                                        continue;
                                    }
                                    // Try native fallback for virtual/interface
                                    let lambda_native_kind = registry.natives_mut().get_kind(
                                        &lambda_info.impl_class,
                                        &lambda_info.impl_method,
                                        &lambda_info.impl_desc,
                                    );
                                    match lambda_native_kind {
                                        Some(HandlerKind::Simple(handler)) => {
                                            #[cfg(feature = "telemetry")]
                                            let _native_start = std::time::Instant::now();
                                            let result = handler(&impl_args, heap, stdout);
                                            #[cfg(feature = "telemetry")]
                                            registry.telemetry.native_boundary.record_call(
                                                &lambda_info.impl_class,
                                                &lambda_info.impl_method,
                                                _native_start.elapsed().as_nanos() as u64,
                                                result.is_err(),
                                            );
                                            let result = match result {
                                                Ok(result) => result,
                                                Err(VmError::JavaException { class_name }) => {
                                                    let exception_ref =
                                                        materialize_java_exception_object(
                                                            registry,
                                                            loader,
                                                            heap,
                                                            &class_name,
                                                        )?;
                                                    propagate_java_exception!(
                                                        class_name,
                                                        exception_ref,
                                                        pc
                                                    );
                                                }
                                                Err(err) => return Err(err),
                                            };
                                            if let Some(val) = result {
                                                frame.push(val)?;
                                            }
                                            idx += 1;
                                            continue;
                                        }
                                        Some(HandlerKind::Callback(handler)) => {
                                            #[cfg(feature = "telemetry")]
                                            let _native_start = std::time::Instant::now();
                                            let mut invoke_cb = create_invoke_cb!(registry, loader);
                                            let result =
                                                handler(&impl_args, heap, stdout, &mut invoke_cb);
                                            #[cfg(feature = "telemetry")]
                                            registry.telemetry.native_boundary.record_call(
                                                &lambda_info.impl_class,
                                                &lambda_info.impl_method,
                                                _native_start.elapsed().as_nanos() as u64,
                                                result.is_err(),
                                            );
                                            let result = match result {
                                                Ok(result) => result,
                                                Err(VmError::JavaException { class_name }) => {
                                                    let exception_ref =
                                                        materialize_java_exception_object(
                                                            registry,
                                                            loader,
                                                            heap,
                                                            &class_name,
                                                        )?;
                                                    propagate_java_exception!(
                                                        class_name,
                                                        exception_ref,
                                                        pc
                                                    );
                                                }
                                                Err(err) => return Err(err),
                                            };
                                            if let Some(val) = result {
                                                frame.push(val)?;
                                            }
                                            idx += 1;
                                            continue;
                                        }
                                        None => {}
                                    }
                                }
                                idx += 1;
                                continue;
                            }
                            // No-op fallback.
                            idx += 1;
                            continue;
                        }
                    }
                };

                // Bytecode execution path — Pattern B: pop directly into locals_buf.
                #[cfg(feature = "telemetry")]
                registry.telemetry.dispatch_resolution.record(
                    &current_class,
                    cp_idx.0,
                    &dispatch_class,
                    dispatch_class != actual_class,
                );
                let (callee_pc_to_idx, callee_frame) = {
                    let ctx = registry.get(&dispatch_class)?;
                    let max_locals = usize::from(ctx.methods[callee_idx].max_locals);
                    let max_stack = usize::from(ctx.methods[callee_idx].max_stack);
                    let pci = std::sync::Arc::clone(&ctx.methods[callee_idx].pc_to_idx);
                    let (mut locals_buf, stack_buf) = frame_pool.acquire();
                    locals_buf.resize(max_locals, Slot::Int(0));
                    if arg_count + 1 > max_locals {
                        return Err(VmError::LocalOutOfBounds {
                            index: arg_count + 1,
                            max_locals,
                        });
                    }
                    // Pop method args in reverse (stack top = last arg) into locals[1..=arg_count].
                    for i in (1..=arg_count).rev() {
                        locals_buf[i] = frame.pop()?;
                    }
                    locals_buf[0] = frame.pop()?; // `this`
                    let f = Frame::from_pool_bufs(locals_buf, stack_buf, max_stack);
                    (pci, f)
                };
                call_stack.push(CallFrame {
                    frame,
                    method_idx,
                    pc_to_idx,
                    resume_idx: idx + 1,
                    class_name: current_class.clone(),
                });
                frame = callee_frame;
                method_idx = callee_idx;
                pc_to_idx = callee_pc_to_idx;
                current_class = dispatch_class;
                instructions = std::sync::Arc::clone(
                    &registry.get(&current_class)?.methods[method_idx].instructions,
                );
                #[cfg(feature = "telemetry")]
                {
                    current_method = registry
                        .get(&current_class)
                        .map(|c| {
                            c.methods
                                .get(method_idx)
                                .map(|m| m.name.clone())
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();
                }
                idx = 0;
                continue;
            }

            // ----------------------------------------------------------------
            // multianewarray — allocate multi-dimensional arrays.
            // ----------------------------------------------------------------
            Instruction::Multianewarray {
                index: cp_idx,
                dimensions,
            } => {
                let element_type = {
                    let ctx = registry.get(&current_class)?;
                    resolve_class_name(&ctx.constant_pool, usize::from(cp_idx.0))?
                };

                // Pop dimension sizes from stack (first popped is rightmost dimension).
                let mut dims: Vec<i32> = Vec::new();
                for _ in 0..*dimensions {
                    dims.push(frame.pop_int()?);
                }
                dims.reverse(); // Now dims[0] is outermost.

                // Check for negative sizes.
                for &d in &dims {
                    if d < 0 {
                        return Err(VmError::NegativeArraySize { size: d });
                    }
                }

                // Recursive allocation helper.
                fn alloc_multi(
                    heap: &mut duke_gc::Heap,
                    dims: &[i32],
                    depth: usize,
                    type_name: &str,
                ) -> u64 {
                    let size = dims[depth] as usize;
                    let r = heap.allocate(type_name.to_string(), size);
                    if depth < dims.len() - 1 {
                        // Not the innermost — fill with references to sub-arrays.
                        let inner_type = &type_name[1..]; // Strip one '[' for inner dimension.
                        for i in 0..size {
                            let inner = alloc_multi(heap, dims, depth + 1, inner_type);
                            heap.get_mut(r).unwrap().fields[i] = Slot::Reference(Some(inner));
                        }
                    }
                    r
                }

                let r = alloc_multi(heap, &dims, 0, &element_type);
                frame.push(Slot::Reference(Some(r)))?;
                if heap.should_gc() {
                    let roots = gather_roots(&frame, &call_stack, registry);
                    heap.collect(&roots);
                    patch_forwarded_slots(&mut frame, &mut call_stack, registry, heap);
                }
            }

            // ---- monitor (no-op, single-threaded) ----
            Instruction::Monitorenter | Instruction::Monitorexit => {
                let _obj = frame.pop()?;
            }

            other => {
                return Err(VmError::Unimplemented {
                    mnemonic: other.mnemonic(),
                });
            }
        }

        #[cfg(feature = "telemetry")]
        {
            let elapsed = _telem_start.elapsed().as_nanos() as u64;
            registry.telemetry.bytecode_cost.record(
                _telem_name,
                &current_class,
                &current_method,
                _telem_pc,
                elapsed,
            );
        }

        idx += 1;
    }
}

/// Saved state of a caller frame suspended during a method call.
struct CallFrame {
    frame: Frame,
    method_idx: usize,
    pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
    resume_idx: usize,
    /// Class that was executing when this frame was pushed.
    class_name: String,
}

/// Build a [`ClassContext`] from a parsed [`duke_classfile::ClassFile`].
///
/// Decodes all methods with a Code attribute and extracts field metadata.
/// Methods without Code (abstract, native) are silently skipped.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn build_class_context(cf: &duke_classfile::ClassFile) -> ClassContext {
    use duke_bytecode::decode;
    use duke_classfile::access_flags::FieldAccessFlags;
    use duke_classfile::types::{AttributeData, CpEntry};

    // Resolve this_class -> class name string.
    let class_name = {
        let entry = cf
            .constant_pool
            .get(cf.this_class.0 as usize)
            .and_then(|e| e.as_ref());
        if let Some(CpEntry::Class { name_index }) = entry {
            match cf
                .constant_pool
                .get(name_index.0 as usize)
                .and_then(|e| e.as_ref())
            {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => String::new(),
            }
        } else {
            String::new()
        }
    };

    let methods = cf
        .methods
        .iter()
        .filter_map(|m| {
            let name = match cf.constant_pool.get(m.name_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let descriptor = match cf.constant_pool.get(m.descriptor_index.0 as usize) {
                Some(Some(CpEntry::Utf8(s))) => s.clone(),
                _ => return None,
            };
            let code = m.attributes.iter().find_map(|a| {
                if let AttributeData::Code(c) = &a.data {
                    Some(c)
                } else {
                    None
                }
            })?;
            let instructions = decode(&code.code).ok()?;
            let exception_table: Vec<ExceptionEntry> = code
                .exception_table
                .iter()
                .map(|e| {
                    let catch_type = if e.catch_type.0 == 0 {
                        None // catch-all (finally)
                    } else {
                        match cf
                            .constant_pool
                            .get(e.catch_type.0 as usize)
                            .and_then(|x| x.as_ref())
                        {
                            Some(CpEntry::Class { name_index }) => {
                                match cf
                                    .constant_pool
                                    .get(name_index.0 as usize)
                                    .and_then(|x| x.as_ref())
                                {
                                    Some(CpEntry::Utf8(s)) => Some(s.clone()),
                                    _ => None,
                                }
                            }
                            _ => None,
                        }
                    };
                    ExceptionEntry {
                        start_pc: e.start_pc,
                        end_pc: e.end_pc,
                        handler_pc: e.handler_pc,
                        catch_type,
                    }
                })
                .collect();
            let pc_to_idx_map: std::collections::HashMap<usize, usize> = instructions
                .iter()
                .enumerate()
                .map(|(i, &(pc, _))| (pc, i))
                .collect();
            Some(MethodEntry {
                name,
                descriptor,
                instructions: instructions.into(),
                max_stack: code.max_stack,
                max_locals: code.max_locals,
                exception_table,
                pc_to_idx: std::sync::Arc::new(pc_to_idx_map),
            })
        })
        .collect();

    let mut fields = Vec::new();
    let mut static_count = 0usize;
    let mut instance_count = 0usize;

    for f in &cf.fields {
        let name = match cf.constant_pool.get(f.name_index.0 as usize) {
            Some(Some(CpEntry::Utf8(s))) => s.clone(),
            _ => continue,
        };
        let descriptor = match cf.constant_pool.get(f.descriptor_index.0 as usize) {
            Some(Some(CpEntry::Utf8(s))) => s.clone(),
            _ => continue,
        };
        let is_static = f.access_flags.contains(FieldAccessFlags::STATIC);
        if is_static {
            static_count += 1;
        } else {
            instance_count += 1;
        }
        fields.push(FieldEntry {
            name,
            descriptor,
            is_static,
        });
    }

    // Resolve super_class: if the index is 0, this is java/lang/Object (no super).
    let super_class = if cf.super_class.0 != 0 {
        resolve_class_name(&cf.constant_pool, cf.super_class.0 as usize).ok()
    } else {
        None
    };

    // Resolve directly-implemented interfaces.
    let interfaces: Vec<String> = cf
        .interfaces
        .iter()
        .filter_map(|idx| resolve_class_name(&cf.constant_pool, idx.0 as usize).ok())
        .collect();

    // Extract BootstrapMethods from class-level attributes.
    let bootstrap_methods = cf
        .attributes
        .iter()
        .find_map(|a| {
            if let AttributeData::BootstrapMethods(entries) = &a.data {
                Some(entries.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    ClassContext {
        class_name,
        super_class,
        interfaces,
        constant_pool: cf.constant_pool.clone(),
        methods,
        fields,
        static_fields: vec![Slot::Int(0); static_count],
        instance_field_count: instance_count,
        bootstrap_methods,
    }
}

/// Resolve a CP Class entry to its name string.
fn resolve_class_name(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Class { name_index }) => {
            match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s.clone()),
                _ => Err(VmError::InvalidCpIndex {
                    index: name_index.0 as usize,
                }),
            }
        }
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

/// Push a constant pool value onto the frame's operand stack.
fn ldc_push(frame: &mut Frame, cp: &[Option<CpEntry>], idx: usize) -> VmResult<()> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Integer(v)) => frame.push(Slot::Int(*v)),
        Some(CpEntry::Float(v)) => frame.push(Slot::Float(*v)),
        Some(CpEntry::Long(v)) => frame.push(Slot::Long(*v)),
        Some(CpEntry::Double(v)) => frame.push(Slot::Double(*v)),
        _ => Err(VmError::InvalidCpIndex { index: idx }),
    }
}

/// Check if `from` is a subtype of `to` (i.e., `from` can be assigned where `to` is expected).
///
/// Walks the class hierarchy from `from` upward through superclasses.
/// Returns `true` if `to` is found in the chain, or if `to` is `"java/lang/Object"`.
/// Return `true` if a reference of type `from` can be used where `to` is expected.
///
/// BFS over the full type graph (superclass + all implemented interfaces at each
/// level), so `String instanceof Comparable` resolves correctly.
fn is_assignable_from(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    from: &str,
    to: &str,
) -> bool {
    if from == to || to == "java/lang/Object" {
        return true;
    }
    // Arrays implement Cloneable and Serializable; everything else is Object.
    if from.starts_with('[') {
        return matches!(to, "java/lang/Cloneable" | "java/io/Serializable");
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    queue.push_back(from.to_string());

    while let Some(current) = queue.pop_front() {
        if !visited.insert(current.clone()) {
            continue;
        }
        let _ = registry.ensure_loaded(&current, loader);
        let (super_class, interfaces) = match registry.get(&current) {
            Ok(ctx) => (ctx.super_class.clone(), ctx.interfaces.clone()),
            Err(_) => continue,
        };
        if let Some(sc) = super_class {
            if sc == to {
                return true;
            }
            queue.push_back(sc);
        }
        for iface in interfaces {
            if iface == to {
                return true;
            }
            queue.push_back(iface);
        }
    }
    false
}

/// Search a method's exception table for a handler matching the given pc and exception class.
///
/// Uses hierarchy-aware type checking: a `catch(Exception)` will match a thrown
/// `RuntimeException` because `RuntimeException` is a subclass of `Exception`.
fn find_exception_handler(
    exception_table: &[ExceptionEntry],
    pc: usize,
    class_name: &str,
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
) -> Option<u16> {
    exception_table.iter().find_map(|entry| {
        let in_range = pc >= entry.start_pc as usize && pc < entry.end_pc as usize;
        #[allow(clippy::option_if_let_else)] // match is clearer with &mut registry
        let type_matches = match &entry.catch_type {
            None => true, // catch-all (finally)
            Some(ct) => is_assignable_from(registry, loader, class_name, ct),
        };
        if in_range && type_matches {
            Some(entry.handler_pc)
        } else {
            None
        }
    })
}

/// Walk the class hierarchy to find a method by name and descriptor.
/// Returns `(class_name_where_found, method_index)` or `None`.
fn resolve_method_in_hierarchy(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    start_class: &str,
    method_name: &str,
    method_desc: &str,
) -> Option<(String, usize)> {
    let mut current = start_class.to_string();
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(current.clone()) {
            return None; // circular — bail
        }
        let _ = registry.ensure_loaded(&current, loader);
        match registry.get(&current) {
            Ok(ctx) => {
                if let Some(idx) = ctx
                    .methods
                    .iter()
                    .position(|m| m.name == method_name && m.descriptor == method_desc)
                {
                    return Some((current, idx));
                }
                match &ctx.super_class {
                    Some(s) => current.clone_from(s),
                    None => return None,
                }
            }
            Err(_) => return None,
        }
    }
}

/// Resolve a constant pool Methodref to (`class_name`, `method_name`, `descriptor`).
fn resolve_methodref(cp: &[Option<CpEntry>], idx: usize) -> VmResult<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(
            CpEntry::Methodref {
                class_index,
                name_and_type_index,
            }
            | CpEntry::InterfaceMethodref {
                class_index,
                name_and_type_index,
            },
        ) => {
            let class_name = match cp.get(class_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Class { name_index }) => {
                    match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    }
                }
                _ => return Err(VmError::InvalidMethodref { index: idx }),
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidMethodref { index: idx }),
                    };
                    Ok((class_name, name, desc))
                }
                _ => Err(VmError::InvalidMethodref { index: nat_idx }),
            }
        }
        _ => Err(VmError::InvalidMethodref { index: idx }),
    }
}

/// Count argument slots in a JVM method descriptor like `(ILjava/lang/String;[I)V`.
fn parse_arg_count(descriptor: &str) -> usize {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut count = 0;
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => count += 1,
            '[' => {
                while chars.peek() == Some(&'[') {
                    chars.next();
                }
                if chars.peek() == Some(&'L') {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2 == ';' {
                            break;
                        }
                    }
                } else {
                    chars.next();
                }
                count += 1;
            }
            'L' => {
                for c2 in chars.by_ref() {
                    if c2 == ';' {
                        break;
                    }
                }
                count += 1;
            }
            _ => {}
        }
    }
    count
}

/// Resolve a constant pool Fieldref to (`class_name`, `field_name`, descriptor).
fn resolve_fieldref(cp: &[Option<CpEntry>], idx: usize) -> VmResult<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Fieldref {
            class_index,
            name_and_type_index,
        }) => {
            let class_name = match cp.get(class_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Class { name_index }) => {
                    match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    }
                }
                _ => return Err(VmError::InvalidFieldref { index: idx }),
            };
            let nat_idx = name_and_type_index.0 as usize;
            match cp.get(nat_idx).and_then(|e| e.as_ref()) {
                Some(CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                }) => {
                    let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    };
                    let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                        Some(CpEntry::Utf8(s)) => s.clone(),
                        _ => return Err(VmError::InvalidFieldref { index: idx }),
                    };
                    Ok((class_name, name, desc))
                }
                _ => Err(VmError::InvalidFieldref { index: nat_idx }),
            }
        }
        _ => Err(VmError::InvalidFieldref { index: idx }),
    }
}

/// Resolve a `MethodHandle` CP entry to (`reference_kind`, `class_name`, `method_name`, descriptor).
fn resolve_method_handle(
    cp: &[Option<CpEntry>],
    cp_idx: usize,
) -> VmResult<(u8, String, String, String)> {
    let (kind, ref_idx) = match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::MethodHandle {
            reference_kind,
            reference_index,
        }) => (*reference_kind, reference_index.0 as usize),
        _ => return Err(VmError::InvalidCpIndex { index: cp_idx }),
    };
    let (class_name, method_name, descriptor) = resolve_methodref(cp, ref_idx)?;
    Ok((kind, class_name, method_name, descriptor))
}

/// Resolve a `NameAndType` CP entry to (name, descriptor).
fn resolve_name_and_type(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<(String, String)> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::NameAndType {
            name_index,
            descriptor_index,
        }) => {
            let name = match cp.get(name_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => {
                    return Err(VmError::InvalidCpIndex {
                        index: name_index.0 as usize,
                    });
                }
            };
            let desc = match cp.get(descriptor_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => s.clone(),
                _ => {
                    return Err(VmError::InvalidCpIndex {
                        index: descriptor_index.0 as usize,
                    });
                }
            };
            Ok((name, desc))
        }
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

/// Resolve a CP String entry to its UTF-8 content. Also handles bare Utf8 entries.
fn resolve_cp_string(cp: &[Option<CpEntry>], cp_idx: usize) -> VmResult<String> {
    match cp.get(cp_idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::String { string_index }) => {
            match cp.get(string_index.0 as usize).and_then(|e| e.as_ref()) {
                Some(CpEntry::Utf8(s)) => Ok(s.clone()),
                _ => Err(VmError::InvalidCpIndex {
                    index: string_index.0 as usize,
                }),
            }
        }
        Some(CpEntry::Utf8(s)) => Ok(s.clone()),
        _ => Err(VmError::InvalidCpIndex { index: cp_idx }),
    }
}

/// Parse argument type descriptors from a JVM method descriptor like `(IZLjava/lang/String;)V`.
/// Returns a Vec of single-char type codes: 'I', 'Z', 'L' (for object refs), '[' (for arrays), etc.
fn parse_arg_types(descriptor: &str) -> Vec<char> {
    let params = descriptor
        .strip_prefix('(')
        .and_then(|s| s.split_once(')'))
        .map_or("", |(p, _)| p);
    let mut types = Vec::new();
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => types.push(c),
            '[' => {
                // Skip array dimensions and element type.
                while chars.peek() == Some(&'[') {
                    chars.next();
                }
                if chars.peek() == Some(&'L') {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2 == ';' {
                            break;
                        }
                    }
                } else {
                    chars.next();
                }
                types.push('[');
            }
            'L' => {
                for c2 in chars.by_ref() {
                    if c2 == ';' {
                        break;
                    }
                }
                types.push('L');
            }
            _ => {}
        }
    }
    types
}

/// Index of a named instance field within ctx.fields (non-static only).
/// Return the correct default [`Slot`] for a field with the given JVM descriptor.
///
/// Per JVMS §2.3/2.4: numeric types default to 0, reference/array types to null.
#[inline]
fn default_slot_for_descriptor(desc: &str) -> Slot {
    match desc.chars().next() {
        Some('J') => Slot::Long(0),
        Some('F') => Slot::Float(0.0),
        Some('D') => Slot::Double(0.0),
        Some('L' | '[') => Slot::Reference(None),
        _ => Slot::Int(0), // I, Z, B, C, S
    }
}

/// reference-typed fields (`L…;` / `[…`), which must be `Reference(None)`.
/// Sum a class's instance fields across its full superclass chain.
fn total_instance_field_count(registry: &ClassRegistry, class_name: &str) -> usize {
    let mut count = registry
        .get(class_name)
        .map(|c| c.instance_field_count)
        .unwrap_or(0);
    let mut sc = registry
        .get(class_name)
        .ok()
        .and_then(|c| c.super_class.clone());
    while let Some(ref s) = sc {
        match registry.get(s) {
            Ok(sctx) => {
                count += sctx.instance_field_count;
                sc = sctx.super_class.clone();
            }
            Err(_) => break,
        }
    }
    count
}

/// Allocate a heap exception object for a native-thrown Java exception.
///
/// Native handlers currently surface Java exceptions as class names. Catch
/// blocks need an object reference on the operand stack, so we materialize a
/// minimal heap object of that class before routing through exception-table
/// dispatch.
fn materialize_java_exception_object(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    class_name: &str,
) -> VmResult<u64> {
    registry.ensure_loaded(class_name, loader)?;
    let exc_ref = heap.allocate(
        class_name.to_string(),
        total_instance_field_count(registry, class_name),
    );
    init_object_fields(registry, heap, exc_ref, class_name);
    Ok(exc_ref)
}

/// After allocating an object on the heap, initialize each field slot to the
/// JVM-spec default for its descriptor.
///
/// `heap.allocate` sets all slots to `Slot::Int(0)`, which is wrong for
/// reference-typed fields (`L...;` / `[...]`), which must be `Reference(None)`.
/// This function walks the full class hierarchy (Object-first) and writes the
/// correct default into every slot that differs from `Int(0)`.
fn init_object_fields(
    registry: &ClassRegistry,
    heap: &mut duke_gc::Heap,
    obj_ref: u64,
    class_name: &str,
) {
    // Collect hierarchy: class_name → … → root
    let mut chain: Vec<String> = Vec::new();
    let mut cur = Some(class_name.to_string());
    while let Some(cls) = cur {
        if let Ok(ctx) = registry.get(&cls) {
            let sc = ctx.super_class.clone();
            chain.push(cls);
            cur = sc;
        } else {
            break;
        }
    }
    chain.reverse(); // Object-first

    let mut slot_idx = 0usize;
    for cls in &chain {
        if let Ok(ctx) = registry.get(cls) {
            for field in ctx.fields.iter().filter(|f| !f.is_static) {
                let default = default_slot_for_descriptor(&field.descriptor);
                // Only write non-Int-zero defaults (avoids an unnecessary mut borrow).
                if !matches!(default, Slot::Int(0))
                    && let Ok(obj) = heap.get_mut(obj_ref)
                    && slot_idx < obj.fields.len()
                {
                    obj.fields[slot_idx] = default;
                }
                slot_idx += 1;
            }
        }
    }
}

/// Compute the absolute slot index of a named instance field within a heap
/// object whose class is `target_class` (or any subclass of it).
///
/// JVM `Fieldref` entries name the access class (often a subclass), not
/// necessarily the declaring class. This function walks the full hierarchy
/// from the root (Object) down to `target_class`, searching each class for
/// the field and accumulating the running slot offset as it goes.
///
/// Layout: root fields occupy the lowest-numbered slots; each subclass
/// appends its fields immediately after its superclass's fields.
fn field_slot_idx(registry: &ClassRegistry, target_class: &str, name: &str) -> VmResult<usize> {
    // Build chain from target_class up to the root, then reverse for Object-first.
    let mut chain: Vec<String> = Vec::new();
    let mut cur = Some(target_class.to_string());
    while let Some(cls) = cur {
        if let Ok(ctx) = registry.get(&cls) {
            let sc = ctx.super_class.clone();
            chain.push(cls);
            cur = sc;
        } else {
            break;
        }
    }
    chain.reverse();

    let mut slot = 0usize;
    for cls in &chain {
        if let Ok(ctx) = registry.get(cls) {
            let instance_fields: Vec<_> = ctx.fields.iter().filter(|f| !f.is_static).collect();
            if let Some(local_idx) = instance_fields.iter().position(|f| f.name == name) {
                return Ok(slot + local_idx);
            }
            slot += instance_fields.len();
        }
    }

    Err(VmError::InvalidFieldref { index: 0 })
}

/// Index of a named static field within `ctx.static_fields`.
fn static_field_idx(ctx: &ClassContext, name: &str) -> VmResult<usize> {
    ctx.fields
        .iter()
        .filter(|f| f.is_static)
        .position(|f| f.name == name)
        .ok_or(VmError::InvalidFieldref { index: 0 })
}

// ---------------------------------------------------------------------------
// StringBuilder natives
// ---------------------------------------------------------------------------

/// Native: `StringBuilder.<init>()V` — initialise empty buffer.
fn native_sb_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
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
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_ascii_digit()))))
}

/// Native: `Character.isLetter(C)Z`
fn native_char_is_letter(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_alphabetic()))))
}

/// Native: `Character.isWhitespace(C)Z`
fn native_char_is_whitespace(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_whitespace()))))
}

/// Native: `Character.isUpperCase(C)Z`
fn native_char_is_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_uppercase()))))
}

/// Native: `Character.isLowerCase(C)Z`
fn native_char_is_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_lowercase()))))
}

/// Native: `Character.toUpperCase(C)C`
fn native_char_to_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
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
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_alphanumeric()))))
}

/// Native: `Character.valueOf(C)Ljava/lang/Character;` — box a char.
fn native_char_valueof(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
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
    invoke: &mut InvokeFn<'_>,
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
            let cmp = invoke(
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
    invoke: &mut InvokeFn<'_>,
) -> VmResult<Option<Slot>> {
    // args[0] = List ref
    let list_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    // Dispatch on the actual runtime class so any List implementation works.
    let class_name = heap.get(list_ref)?.class_name.clone();
    invoke(
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
) -> VmResult<Option<Slot>> {
    Ok(None)
}

/// Native: `ArrayListIterator.hasNext()Z`
fn native_arraylist_iter_hasnext(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
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

// ---------------------------------------------------------------------------
// GC root gathering
// ---------------------------------------------------------------------------

/// Collect all live Slot values from the interpreter's current execution state.
/// The GC uses these as the root set for reachability analysis.
fn gather_roots(
    frame: &duke_runtime::Frame,
    call_stack: &[CallFrame],
    registry: &ClassRegistry,
) -> Vec<duke_runtime::Slot> {
    let mut roots = Vec::new();
    roots.extend(frame.slots());
    for cf in call_stack {
        roots.extend(cf.frame.slots());
    }
    for ctx in registry.all_classes() {
        roots.extend(ctx.static_fields.iter().copied());
    }
    roots
}

/// Apply GC forwarding pointers to all live interpreter slots after a minor
/// collection. Must be called immediately after `heap.collect()` returns so
/// that stale young-gen references are updated to their new locations.
fn patch_forwarded_slots(
    frame: &mut duke_runtime::Frame,
    call_stack: &mut [CallFrame],
    registry: &mut ClassRegistry,
    heap: &duke_gc::Heap,
) {
    for slot in frame.slots_mut() {
        heap.apply_forward(slot);
    }
    for cf in call_stack.iter_mut() {
        for slot in cf.frame.slots_mut() {
            heap.apply_forward(slot);
        }
    }
    for ctx in registry.all_classes_mut() {
        for slot in &mut ctx.static_fields {
            heap.apply_forward(slot);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
            _invoke: &mut InvokeFn<'_>,
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
        ) -> VmResult<Option<Slot>> {
            Ok(Some(Slot::Int(99)))
        }
        let mut natives = NativeRegistry::new();
        natives.register("Foo", "bar", "(I)I", dummy_handler);
        let handler = natives.get("Foo", "bar", "(I)I");
        assert!(handler.is_some());
        let mut heap = duke_gc::Heap::new();
        let mut out: Vec<u8> = Vec::new();
        let result = handler.unwrap()(&[Slot::Int(1)], &mut heap, &mut out).unwrap();
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
        execute_class(
            &mut registry,
            &loader,
            &mut heap,
            &mut out,
            &entry_class,
            method_name,
            descriptor,
            &arg_slots,
        )
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
            |_args, _heap, _out| Ok(Some(Slot::Int(42))),
        );

        // Callback native: invokes the helper and returns its result.
        registry.natives_mut().register_callback(
            "duke/test/Caller",
            "call",
            "()I",
            |_args, heap, output, invoke| {
                CALLED.store(true, std::sync::atomic::Ordering::SeqCst);
                invoke(heap, output, "duke/test/Helper", "answer", "()I", vec![])
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
            |args, heap, _output, _invoke| {
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
            |args, heap, _output, _invoke| {
                CALLED.store(true, Ordering::SeqCst);
                // args[0] is `this` (the Integer object); fields[0] holds the int.
                let r = match &args[0] {
                    Slot::Reference(Some(r)) => *r,
                    _ => return Err(VmError::NullPointerException),
                };
                let val = heap.get(r)?.fields[0];
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
            |_args, _heap, _output, _invoke| {
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
            |args, _heap, _out| Ok(Some(args[0])),
        );

        // Override String.length with a Callback.  This replaces the Simple
        // handler that bootstrap_stdlib registered, so the lambda SAM fallback
        // (Site 5) must route through the Callback arm to fire at all.
        registry.natives_mut().register_callback(
            "java/lang/String",
            "length",
            "()I",
            |args, heap, _output, _invoke| {
                CALLED.store(true, Ordering::SeqCst);
                // args[0] is `this` (the captured String reference).
                let r = match &args[0] {
                    Slot::Reference(Some(r)) => *r,
                    _ => return Err(VmError::NullPointerException),
                };
                let len = i32::try_from(heap.get(r)?.string_value.as_deref().unwrap_or("").len())
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
        let (locals, stack) = pool.acquire();
        assert!(locals.is_empty());
        assert!(stack.is_empty());
    }

    #[test]
    fn frame_pool_release_and_reacquire_returns_pooled_bufs() {
        let mut pool = FramePool::new();
        let locals = vec![Slot::Int(1), Slot::Int(2)];
        let stack = vec![];
        pool.release(locals, stack);
        let (locals2, stack2) = pool.acquire();
        assert_eq!(locals2.len(), 2);
        assert!(stack2.is_empty());
        // Pool should be empty again
        let (locals3, _) = pool.acquire();
        assert!(locals3.is_empty());
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
        let mut invoke_fn = |_heap: &mut duke_gc::Heap,
                             _output: &mut dyn std::io::Write,
                             _class: &str,
                             _method: &str,
                             _desc: &str,
                             _args: Vec<Slot>|
         -> VmResult<Option<Slot>> { Ok(None) };
        let err = array_list_sort(&args, &mut heap, &mut sink, &mut invoke_fn).unwrap_err();
        assert!(
            matches!(err, VmError::NegativeArraySize { .. }),
            "expected NegativeArraySize, got {err:?}"
        );
    }
}
