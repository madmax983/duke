//! Core standard library bootstrapper and minimal Java environment setup.
use crate::context::{ClassContext, ClassLoadSource, FieldEntry};
use crate::registry::ClassRegistry;
#[allow(clippy::wildcard_imports)]
use crate::*;
use duke_runtime::Slot;

fn atomic_context(name: &str, super_class: &str, value_descriptor: &str) -> ClassContext {
    ClassContext {
        class_name: name.to_string(),
        super_class: Some(super_class.to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "value".to_string(),
            descriptor: value_descriptor.to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    }
}

fn empty_synthetic_context(name: &str, super_class: &str) -> ClassContext {
    ClassContext {
        class_name: name.to_string(),
        super_class: Some(super_class.to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    }
}

fn synthetic_field(name: &str, descriptor: &str, is_static: bool) -> FieldEntry {
    FieldEntry {
        name: name.to_string(),
        descriptor: descriptor.to_string(),
        is_static,
    }
}

fn jul_allocate_level(heap: &mut duke_gc::Heap, name: &str, value: i32) -> Slot {
    let level_ref = heap.allocate("java/util/logging/Level".to_string(), 1);
    if let Ok(level) = heap.get_mut(level_ref) {
        level.fields[0] = Slot::Int(value);
        level.string_value = Some(name.to_string());
    }
    Slot::Reference(Some(level_ref))
}

fn jul_allocate_simple_formatter(heap: &mut duke_gc::Heap) -> Slot {
    Slot::Reference(Some(
        heap.allocate("java/util/logging/SimpleFormatter".to_string(), 0),
    ))
}

fn jul_allocate_console_handler(heap: &mut duke_gc::Heap, level_slot: Slot) -> Slot {
    let handler_ref = heap.allocate("java/util/logging/ConsoleHandler".to_string(), 2);
    let formatter = jul_allocate_simple_formatter(heap);
    if let Ok(handler) = heap.get_mut(handler_ref) {
        handler.fields[0] = level_slot;
        handler.fields[1] = formatter;
    }
    Slot::Reference(Some(handler_ref))
}

fn jul_allocate_logger(
    heap: &mut duke_gc::Heap,
    name: Option<&str>,
    level_slot: Slot,
    use_parent_handlers: bool,
    parent_slot: Slot,
    handlers: &[Slot],
) -> Slot {
    const LOGGER_HANDLERS_START: usize = 5;

    let logger_ref = heap.allocate(
        "java/util/logging/Logger".to_string(),
        LOGGER_HANDLERS_START + handlers.len(),
    );
    let name_slot = name.map_or(Slot::Reference(None), |logger_name| {
        Slot::Reference(Some(heap.allocate_string(logger_name.to_string())))
    });
    if let Ok(logger) = heap.get_mut(logger_ref) {
        logger.fields[0] = name_slot;
        logger.fields[1] = level_slot;
        logger.fields[2] = Slot::Int(i32::from(use_parent_handlers));
        logger.fields[3] = parent_slot;
        logger.fields[4] = Slot::Int(i32::try_from(handlers.len()).unwrap_or(i32::MAX));
        for (idx, handler) in handlers.iter().copied().enumerate() {
            logger.fields[LOGGER_HANDLERS_START + idx] = handler;
        }
    }
    Slot::Reference(Some(logger_ref))
}

fn jul_bootstrap_insert_logger(
    heap: &mut duke_gc::Heap,
    manager_ref: u64,
    name: &str,
    logger_slot: Slot,
) {
    let name_slot = Slot::Reference(Some(heap.allocate_string(name.to_string())));
    if let Ok(manager) = heap.get_mut(manager_ref) {
        let next_count = match manager.fields.get(1) {
            Some(Slot::Int(count)) => count.saturating_add(1),
            _ => 1,
        };
        manager.fields[1] = Slot::Int(next_count);
        manager.fields.push(name_slot);
        manager.fields.push(logger_slot);
    }
}

/// Registers Duke's deliberately small `java.util.logging` surface.
///
/// `SimpleFormatter` intentionally diverges from the JDK's two-line default:
/// the native formatter emits one line, `<level>: <message>`, so class
/// initialization logging goes somewhere visible without pulling in the full
/// JUL configuration stack.
#[allow(clippy::too_many_lines)]
fn register_jul_stdlib(registry: &mut ClassRegistry, heap: &mut duke_gc::Heap) {
    let level_specs = [
        ("SEVERE", 1000),
        ("WARNING", 900),
        ("INFO", 800),
        ("CONFIG", 700),
        ("FINE", 500),
        ("FINER", 400),
        ("FINEST", 300),
        ("ALL", i32::MIN),
        ("OFF", i32::MAX),
    ];
    let mut level_fields: Vec<FieldEntry> = level_specs
        .iter()
        .map(|(name, _)| synthetic_field(name, "Ljava/util/logging/Level;", true))
        .collect();
    level_fields.push(synthetic_field("value", "I", false));
    let level_static_fields: Vec<Slot> = level_specs
        .iter()
        .map(|(name, value)| jul_allocate_level(heap, name, *value))
        .collect();
    let info_level = level_static_fields[2];
    let all_level = level_static_fields[7];

    registry.register(ClassContext {
        class_name: "java/util/logging/Level".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: level_fields,
        static_fields: level_static_fields,
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    registry.register(ClassContext {
        class_name: "java/util/logging/Logger".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            synthetic_field("name", "Ljava/lang/String;", false),
            synthetic_field("level", "Ljava/util/logging/Level;", false),
            synthetic_field("useParentHandlers", "Z", false),
            synthetic_field("parent", "Ljava/util/logging/Logger;", false),
            synthetic_field("handlerCount", "I", false),
        ],
        static_fields: Vec::new(),
        instance_field_count: 5,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    registry.register(ClassContext {
        class_name: "java/util/logging/Handler".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            synthetic_field("level", "Ljava/util/logging/Level;", false),
            synthetic_field("formatter", "Ljava/util/logging/Formatter;", false),
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });
    registry.register(empty_synthetic_context(
        "java/util/logging/StreamHandler",
        "java/util/logging/Handler",
    ));
    registry.register(empty_synthetic_context(
        "java/util/logging/ConsoleHandler",
        "java/util/logging/StreamHandler",
    ));
    registry.register(empty_synthetic_context(
        "java/util/logging/Formatter",
        "java/lang/Object",
    ));
    registry.register(empty_synthetic_context(
        "java/util/logging/SimpleFormatter",
        "java/util/logging/Formatter",
    ));

    registry.register(ClassContext {
        class_name: "java/util/logging/LogRecord".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            synthetic_field("level", "Ljava/util/logging/Level;", false),
            synthetic_field("message", "Ljava/lang/String;", false),
            synthetic_field("loggerName", "Ljava/lang/String;", false),
            synthetic_field("thrown", "Ljava/lang/Throwable;", false),
            synthetic_field("millis", "J", false),
            synthetic_field("parameters", "[Ljava/lang/Object;", false),
        ],
        static_fields: Vec::new(),
        instance_field_count: 6,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    registry.register(empty_synthetic_context(
        "java/util/Enumeration",
        "java/lang/Object",
    ));
    registry.register(ClassContext {
        class_name: "duke/util/JulLoggerNameEnumeration".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            synthetic_field("index", "I", false),
            synthetic_field("count", "I", false),
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/Enumeration".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });
    registry.register(ClassContext {
        class_name: "duke/util/ResourceEnumeration".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            synthetic_field("index", "I", false),
            synthetic_field("count", "I", false),
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/Enumeration".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    let root_handler = jul_allocate_console_handler(heap, all_level);
    let root_logger = jul_allocate_logger(
        heap,
        Some(""),
        info_level,
        false,
        Slot::Reference(None),
        &[root_handler],
    );
    let global_logger = jul_allocate_logger(
        heap,
        Some("global"),
        Slot::Reference(None),
        true,
        root_logger,
        &[],
    );

    let manager_ref = heap.allocate("java/util/logging/LogManager".to_string(), 2);
    if let Ok(manager) = heap.get_mut(manager_ref) {
        manager.string_value = Some("singleton".to_string());
        manager.fields[0] = root_logger;
        manager.fields[1] = Slot::Int(0);
    }
    jul_bootstrap_insert_logger(heap, manager_ref, "", root_logger);
    jul_bootstrap_insert_logger(heap, manager_ref, "global", global_logger);

    registry.register(ClassContext {
        class_name: "java/util/logging/LogManager".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            synthetic_field("INSTANCE", "Ljava/util/logging/LogManager;", true),
            synthetic_field("rootLogger", "Ljava/util/logging/Logger;", false),
            synthetic_field("loggerCount", "I", false),
        ],
        static_fields: vec![Slot::Reference(Some(manager_ref))],
        instance_field_count: 2,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    registry.natives_mut().register(
        "java/util/logging/Level",
        "intValue",
        "()I",
        native_jul_level_int_value,
    );
    for method in ["getName", "toString"] {
        registry.natives_mut().register(
            "java/util/logging/Level",
            method,
            "()Ljava/lang/String;",
            native_jul_level_get_name,
        );
    }
    registry.natives_mut().register(
        "java/util/logging/Level",
        "parse",
        "(Ljava/lang/String;)Ljava/util/logging/Level;",
        native_jul_level_parse,
    );

    registry.natives_mut().register(
        "java/util/logging/Logger",
        "getLogger",
        "(Ljava/lang/String;)Ljava/util/logging/Logger;",
        native_jul_logger_get_logger,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "getLogger",
        "(Ljava/lang/String;Ljava/lang/String;)Ljava/util/logging/Logger;",
        native_jul_logger_get_logger_with_bundle,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "getGlobal",
        "()Ljava/util/logging/Logger;",
        native_jul_logger_get_global,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "getAnonymousLogger",
        "()Ljava/util/logging/Logger;",
        native_jul_logger_get_anonymous_logger,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "getName",
        "()Ljava/lang/String;",
        native_jul_logger_get_name,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "getLevel",
        "()Ljava/util/logging/Level;",
        native_jul_logger_get_level,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "setLevel",
        "(Ljava/util/logging/Level;)V",
        native_jul_logger_set_level,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "isLoggable",
        "(Ljava/util/logging/Level;)Z",
        native_jul_logger_is_loggable,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "log",
        "(Ljava/util/logging/Level;Ljava/lang/String;)V",
        native_jul_logger_log,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "log",
        "(Ljava/util/logging/Level;Ljava/lang/String;Ljava/lang/Object;)V",
        native_jul_logger_log_object,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "log",
        "(Ljava/util/logging/Level;Ljava/lang/String;[Ljava/lang/Object;)V",
        native_jul_logger_log_object_array,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "log",
        "(Ljava/util/logging/Level;Ljava/lang/String;Ljava/lang/Throwable;)V",
        native_jul_logger_log_throwable,
    );
    for (method, handler) in [
        ("severe", native_jul_logger_severe as NativeHandler),
        ("warning", native_jul_logger_warning),
        ("info", native_jul_logger_info),
        ("config", native_jul_logger_config),
        ("fine", native_jul_logger_fine),
        ("finer", native_jul_logger_finer),
        ("finest", native_jul_logger_finest),
    ] {
        registry.natives_mut().register(
            "java/util/logging/Logger",
            method,
            "(Ljava/lang/String;)V",
            handler,
        );
    }
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "entering",
        "(Ljava/lang/String;Ljava/lang/String;)V",
        native_jul_logger_entering,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "exiting",
        "(Ljava/lang/String;Ljava/lang/String;)V",
        native_jul_logger_exiting,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "throwing",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/Throwable;)V",
        native_jul_logger_throwing,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "addHandler",
        "(Ljava/util/logging/Handler;)V",
        native_jul_logger_add_handler,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "removeHandler",
        "(Ljava/util/logging/Handler;)V",
        native_jul_logger_remove_handler,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "getHandlers",
        "()[Ljava/util/logging/Handler;",
        native_jul_logger_get_handlers,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "setUseParentHandlers",
        "(Z)V",
        native_jul_logger_set_use_parent_handlers,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "getUseParentHandlers",
        "()Z",
        native_jul_logger_get_use_parent_handlers,
    );
    registry.natives_mut().register(
        "java/util/logging/Logger",
        "getParent",
        "()Ljava/util/logging/Logger;",
        native_jul_logger_get_parent,
    );

    registry.natives_mut().register(
        "java/util/logging/LogManager",
        "getLogManager",
        "()Ljava/util/logging/LogManager;",
        native_jul_log_manager_get_log_manager,
    );
    registry.natives_mut().register(
        "java/util/logging/LogManager",
        "getLogger",
        "(Ljava/lang/String;)Ljava/util/logging/Logger;",
        native_jul_log_manager_get_logger,
    );
    registry.natives_mut().register(
        "java/util/logging/LogManager",
        "getLoggerNames",
        "()Ljava/util/Enumeration;",
        native_jul_log_manager_get_logger_names,
    );
    registry.natives_mut().register(
        "java/util/logging/LogManager",
        "readConfiguration",
        "()V",
        native_jul_noop_void,
    );
    registry.natives_mut().register_callback(
        "java/util/logging/LogManager",
        "readConfiguration",
        "(Ljava/io/InputStream;)V",
        native_jul_log_manager_read_configuration_stream,
    );
    registry.natives_mut().register(
        "java/util/logging/LogManager",
        "reset",
        "()V",
        native_jul_noop_void,
    );
    registry.natives_mut().register(
        "java/util/logging/LogManager",
        "addLogger",
        "(Ljava/util/logging/Logger;)Z",
        native_jul_log_manager_add_logger,
    );

    for class_name in [
        "java/util/logging/Handler",
        "java/util/logging/StreamHandler",
        "java/util/logging/ConsoleHandler",
    ] {
        registry.natives_mut().register(
            class_name,
            "<init>",
            "()V",
            if class_name == "java/util/logging/ConsoleHandler" {
                native_jul_console_handler_init
            } else {
                native_jul_handler_init
            },
        );
        registry.natives_mut().register(
            class_name,
            "publish",
            "(Ljava/util/logging/LogRecord;)V",
            if class_name == "java/util/logging/ConsoleHandler" {
                native_jul_console_handler_publish
            } else {
                native_jul_handler_publish
            },
        );
        registry
            .natives_mut()
            .register(class_name, "flush", "()V", native_jul_noop_void);
        registry
            .natives_mut()
            .register(class_name, "close", "()V", native_jul_noop_void);
        registry.natives_mut().register(
            class_name,
            "setLevel",
            "(Ljava/util/logging/Level;)V",
            native_jul_handler_set_level,
        );
        registry.natives_mut().register(
            class_name,
            "getLevel",
            "()Ljava/util/logging/Level;",
            native_jul_handler_get_level,
        );
        registry.natives_mut().register(
            class_name,
            "setFormatter",
            "(Ljava/util/logging/Formatter;)V",
            native_jul_handler_set_formatter,
        );
    }

    for class_name in [
        "java/util/logging/Formatter",
        "java/util/logging/SimpleFormatter",
    ] {
        registry
            .natives_mut()
            .register(class_name, "<init>", "()V", native_jul_noop_void);
        registry.natives_mut().register(
            class_name,
            "format",
            "(Ljava/util/logging/LogRecord;)Ljava/lang/String;",
            native_jul_simple_formatter_format,
        );
    }

    registry.natives_mut().register(
        "java/util/logging/LogRecord",
        "<init>",
        "(Ljava/util/logging/Level;Ljava/lang/String;)V",
        native_jul_log_record_init,
    );
    registry.natives_mut().register(
        "java/util/logging/LogRecord",
        "getLevel",
        "()Ljava/util/logging/Level;",
        native_jul_log_record_get_level,
    );
    registry.natives_mut().register(
        "java/util/logging/LogRecord",
        "setLevel",
        "(Ljava/util/logging/Level;)V",
        native_jul_log_record_set_level,
    );
    registry.natives_mut().register(
        "java/util/logging/LogRecord",
        "getMessage",
        "()Ljava/lang/String;",
        native_jul_log_record_get_message,
    );
    registry.natives_mut().register(
        "java/util/logging/LogRecord",
        "setMessage",
        "(Ljava/lang/String;)V",
        native_jul_log_record_set_message,
    );
    registry.natives_mut().register(
        "java/util/logging/LogRecord",
        "getLoggerName",
        "()Ljava/lang/String;",
        native_jul_log_record_get_logger_name,
    );
    registry.natives_mut().register(
        "java/util/logging/LogRecord",
        "setLoggerName",
        "(Ljava/lang/String;)V",
        native_jul_log_record_set_logger_name,
    );
    registry.natives_mut().register(
        "java/util/logging/LogRecord",
        "getThrown",
        "()Ljava/lang/Throwable;",
        native_jul_log_record_get_thrown,
    );
    registry.natives_mut().register(
        "java/util/logging/LogRecord",
        "setThrown",
        "(Ljava/lang/Throwable;)V",
        native_jul_log_record_set_thrown,
    );
    registry.natives_mut().register(
        "java/util/logging/LogRecord",
        "getMillis",
        "()J",
        native_jul_log_record_get_millis,
    );
    registry.natives_mut().register(
        "java/util/logging/LogRecord",
        "getParameters",
        "()[Ljava/lang/Object;",
        native_jul_log_record_get_parameters,
    );
    registry.natives_mut().register(
        "java/util/logging/LogRecord",
        "setParameters",
        "([Ljava/lang/Object;)V",
        native_jul_log_record_set_parameters,
    );

    for class_name in [
        "java/util/Enumeration",
        "duke/util/JulLoggerNameEnumeration",
    ] {
        registry.natives_mut().register(
            class_name,
            "hasMoreElements",
            "()Z",
            native_jul_logger_names_has_more_elements,
        );
        registry.natives_mut().register(
            class_name,
            "nextElement",
            "()Ljava/lang/Object;",
            native_jul_logger_names_next_element,
        );
    }
    registry.natives_mut().register(
        "duke/util/ResourceEnumeration",
        "hasMoreElements",
        "()Z",
        native_resource_enumeration_has_more_elements,
    );
    registry.natives_mut().register(
        "duke/util/ResourceEnumeration",
        "nextElement",
        "()Ljava/lang/Object;",
        native_resource_enumeration_next_element,
    );
}

fn register_charset_classes(registry: &mut ClassRegistry, heap: &mut duke_gc::Heap) {
    let charset_ctx = empty_synthetic_context("java/nio/charset/Charset", "java/lang/Object");
    registry.register(charset_ctx);

    let standard_charset_fields = [
        ("UTF_8", "UTF-8"),
        ("UTF_16", "UTF-16"),
        ("UTF_16BE", "UTF-16BE"),
        ("UTF_16LE", "UTF-16LE"),
        ("US_ASCII", "US-ASCII"),
        ("ISO_8859_1", "ISO-8859-1"),
    ];
    let standard_ctx = ClassContext {
        class_name: "java/nio/charset/StandardCharsets".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: standard_charset_fields
            .iter()
            .map(|(field_name, _)| FieldEntry {
                name: (*field_name).to_string(),
                descriptor: "Ljava/nio/charset/Charset;".to_string(),
                is_static: true,
            })
            .collect(),
        static_fields: standard_charset_fields
            .iter()
            .map(|(_, canonical_name)| {
                Slot::Reference(Some(allocate_standard_charset(heap, canonical_name)))
            })
            .collect(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(standard_ctx);

    registry.register(empty_synthetic_context(
        "java/nio/charset/UnsupportedCharsetException",
        "java/lang/IllegalArgumentException",
    ));
    registry.register(empty_synthetic_context(
        "java/io/UnsupportedEncodingException",
        "java/io/IOException",
    ));
}

fn register_charset_natives(registry: &mut ClassRegistry) {
    registry.natives_mut().register(
        "java/nio/charset/Charset",
        "forName",
        "(Ljava/lang/String;)Ljava/nio/charset/Charset;",
        native_charset_for_name,
    );
    registry.natives_mut().register(
        "java/nio/charset/Charset",
        "defaultCharset",
        "()Ljava/nio/charset/Charset;",
        native_charset_default_charset,
    );
    for method in ["name", "displayName", "toString"] {
        registry.natives_mut().register(
            "java/nio/charset/Charset",
            method,
            "()Ljava/lang/String;",
            native_charset_name,
        );
    }
    registry.natives_mut().register(
        "java/nio/charset/Charset",
        "isRegistered",
        "()Z",
        native_charset_is_registered,
    );
    registry.natives_mut().register(
        "java/nio/charset/Charset",
        "equals",
        "(Ljava/lang/Object;)Z",
        native_charset_equals,
    );
    registry.natives_mut().register(
        "java/nio/charset/Charset",
        "hashCode",
        "()I",
        native_charset_hash_code,
    );
}

fn register_string_byte_conversion_natives(registry: &mut ClassRegistry) {
    registry.natives_mut().register(
        "java/lang/String",
        "getBytes",
        "()[B",
        native_string_get_bytes_default,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "getBytes",
        "(Ljava/lang/String;)[B",
        native_string_get_bytes_named,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "getBytes",
        "(Ljava/nio/charset/Charset;)[B",
        native_string_get_bytes_charset,
    );

    for (descriptor, handler) in [
        ("([B)V", native_string_init_bytes_default as NativeHandler),
        ("([BII)V", native_string_init_bytes_default_range),
        ("([BLjava/lang/String;)V", native_string_init_bytes_named),
        (
            "([BLjava/nio/charset/Charset;)V",
            native_string_init_bytes_charset,
        ),
        (
            "([BIILjava/nio/charset/Charset;)V",
            native_string_init_bytes_range_charset,
        ),
        (
            "([BIILjava/lang/String;)V",
            native_string_init_bytes_range_named,
        ),
    ] {
        registry
            .natives_mut()
            .register("java/lang/String", "<init>", descriptor, handler);
    }
}

/// Registers `java.nio.charset` synthetics and the matching `String` byte conversions.
fn register_charset_stdlib(registry: &mut ClassRegistry, heap: &mut duke_gc::Heap) {
    register_charset_classes(registry, heap);
    register_charset_natives(registry);
    register_string_byte_conversion_natives(registry);
}

fn register_base64_stdlib(registry: &mut ClassRegistry) {
    registry.register(empty_synthetic_context(
        "java/util/Base64",
        "java/lang/Object",
    ));

    for class_name in ["java/util/Base64$Encoder", "java/util/Base64$Decoder"] {
        registry.register(ClassContext {
            class_name: class_name.to_string(),
            super_class: Some("java/lang/Object".to_string()),
            constant_pool: Vec::new(),
            methods: Vec::new(),
            fields: vec![FieldEntry {
                name: "variant".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            }],
            static_fields: Vec::new(),
            instance_field_count: 1,
            interfaces: Vec::new(),
            bootstrap_methods: Vec::new(),
            load_source: ClassLoadSource::Synthetic,
        });
    }

    for (method, descriptor, handler) in [
        (
            "getEncoder",
            "()Ljava/util/Base64$Encoder;",
            native_base64_get_encoder as NativeHandler,
        ),
        (
            "getMimeEncoder",
            "()Ljava/util/Base64$Encoder;",
            native_base64_get_mime_encoder,
        ),
        (
            "getUrlEncoder",
            "()Ljava/util/Base64$Encoder;",
            native_base64_get_url_encoder,
        ),
        (
            "getDecoder",
            "()Ljava/util/Base64$Decoder;",
            native_base64_get_decoder,
        ),
        (
            "getMimeDecoder",
            "()Ljava/util/Base64$Decoder;",
            native_base64_get_mime_decoder,
        ),
        (
            "getUrlDecoder",
            "()Ljava/util/Base64$Decoder;",
            native_base64_get_url_decoder,
        ),
    ] {
        registry
            .natives_mut()
            .register("java/util/Base64", method, descriptor, handler);
    }

    registry.natives_mut().register(
        "java/util/Base64$Encoder",
        "encodeToString",
        "([B)Ljava/lang/String;",
        native_base64_encoder_encode_to_string,
    );
    registry.natives_mut().register(
        "java/util/Base64$Encoder",
        "encode",
        "([B)[B",
        native_base64_encoder_encode,
    );
    registry.natives_mut().register(
        "java/util/Base64$Decoder",
        "decode",
        "(Ljava/lang/String;)[B",
        native_base64_decoder_decode_string,
    );
    registry.natives_mut().register(
        "java/util/Base64$Decoder",
        "decode",
        "([B)[B",
        native_base64_decoder_decode_bytes,
    );
}

/// Registers synthetic `java.util.concurrent.atomic` classes.
///
/// `AtomicReference.compareAndSet` is deliberately identity-based: it compares
/// `Slot::Reference` addresses, not `Object.equals`, matching `HotSpot`'s object
/// CAS behavior.
#[allow(clippy::too_many_lines)]
fn register_atomic_stdlib(registry: &mut ClassRegistry) {
    let number_ctx = ClassContext {
        class_name: "java/lang/Number".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(number_ctx);

    registry.register(atomic_context(
        "java/util/concurrent/atomic/AtomicInteger",
        "java/lang/Number",
        "I",
    ));
    registry.register(atomic_context(
        "java/util/concurrent/atomic/AtomicLong",
        "java/lang/Number",
        "J",
    ));
    registry.register(atomic_context(
        "java/util/concurrent/atomic/AtomicReference",
        "java/lang/Object",
        "Ljava/lang/Object;",
    ));
    registry.register(atomic_context(
        "java/util/concurrent/atomic/AtomicBoolean",
        "java/lang/Object",
        "Z",
    ));

    for (method, descriptor, handler) in [
        ("<init>", "()V", native_atomic_integer_init as NativeHandler),
        ("<init>", "(I)V", native_atomic_integer_init_value),
        ("get", "()I", native_atomic_integer_get),
        ("set", "(I)V", native_atomic_integer_set),
        ("lazySet", "(I)V", native_atomic_integer_set),
        ("getAndSet", "(I)I", native_atomic_integer_get_and_set),
        (
            "compareAndSet",
            "(II)Z",
            native_atomic_integer_compare_and_set,
        ),
        (
            "weakCompareAndSet",
            "(II)Z",
            native_atomic_integer_compare_and_set,
        ),
        (
            "getAndIncrement",
            "()I",
            native_atomic_integer_get_and_increment,
        ),
        (
            "getAndDecrement",
            "()I",
            native_atomic_integer_get_and_decrement,
        ),
        ("getAndAdd", "(I)I", native_atomic_integer_get_and_add),
        (
            "incrementAndGet",
            "()I",
            native_atomic_integer_increment_and_get,
        ),
        (
            "decrementAndGet",
            "()I",
            native_atomic_integer_decrement_and_get,
        ),
        ("addAndGet", "(I)I", native_atomic_integer_add_and_get),
        ("intValue", "()I", native_atomic_integer_get),
        ("longValue", "()J", native_atomic_integer_long_value),
        ("floatValue", "()F", native_atomic_integer_float_value),
        ("doubleValue", "()D", native_atomic_integer_double_value),
        (
            "toString",
            "()Ljava/lang/String;",
            native_atomic_integer_to_string,
        ),
    ] {
        registry.natives_mut().register(
            "java/util/concurrent/atomic/AtomicInteger",
            method,
            descriptor,
            handler,
        );
    }

    for (method, descriptor, handler) in [
        ("<init>", "()V", native_atomic_long_init as NativeHandler),
        ("<init>", "(J)V", native_atomic_long_init_value),
        ("get", "()J", native_atomic_long_get),
        ("set", "(J)V", native_atomic_long_set),
        ("lazySet", "(J)V", native_atomic_long_set),
        ("getAndSet", "(J)J", native_atomic_long_get_and_set),
        ("compareAndSet", "(JJ)Z", native_atomic_long_compare_and_set),
        (
            "weakCompareAndSet",
            "(JJ)Z",
            native_atomic_long_compare_and_set,
        ),
        (
            "getAndIncrement",
            "()J",
            native_atomic_long_get_and_increment,
        ),
        (
            "getAndDecrement",
            "()J",
            native_atomic_long_get_and_decrement,
        ),
        ("getAndAdd", "(J)J", native_atomic_long_get_and_add),
        (
            "incrementAndGet",
            "()J",
            native_atomic_long_increment_and_get,
        ),
        (
            "decrementAndGet",
            "()J",
            native_atomic_long_decrement_and_get,
        ),
        ("addAndGet", "(J)J", native_atomic_long_add_and_get),
        ("intValue", "()I", native_atomic_long_int_value),
        ("longValue", "()J", native_atomic_long_get),
        ("floatValue", "()F", native_atomic_long_float_value),
        ("doubleValue", "()D", native_atomic_long_double_value),
        (
            "toString",
            "()Ljava/lang/String;",
            native_atomic_long_to_string,
        ),
    ] {
        registry.natives_mut().register(
            "java/util/concurrent/atomic/AtomicLong",
            method,
            descriptor,
            handler,
        );
    }

    for (method, descriptor, handler) in [
        (
            "<init>",
            "()V",
            native_atomic_reference_init as NativeHandler,
        ),
        (
            "<init>",
            "(Ljava/lang/Object;)V",
            native_atomic_reference_init_value,
        ),
        ("get", "()Ljava/lang/Object;", native_atomic_reference_get),
        ("set", "(Ljava/lang/Object;)V", native_atomic_reference_set),
        (
            "lazySet",
            "(Ljava/lang/Object;)V",
            native_atomic_reference_set,
        ),
        (
            "getAndSet",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            native_atomic_reference_get_and_set,
        ),
        (
            "compareAndSet",
            "(Ljava/lang/Object;Ljava/lang/Object;)Z",
            native_atomic_reference_compare_and_set,
        ),
        (
            "weakCompareAndSet",
            "(Ljava/lang/Object;Ljava/lang/Object;)Z",
            native_atomic_reference_compare_and_set,
        ),
        (
            "toString",
            "()Ljava/lang/String;",
            native_atomic_reference_to_string,
        ),
    ] {
        registry.natives_mut().register(
            "java/util/concurrent/atomic/AtomicReference",
            method,
            descriptor,
            handler,
        );
    }

    for (method, descriptor, handler) in [
        ("<init>", "()V", native_atomic_boolean_init as NativeHandler),
        ("<init>", "(Z)V", native_atomic_boolean_init_value),
        ("get", "()Z", native_atomic_boolean_get),
        ("set", "(Z)V", native_atomic_boolean_set),
        (
            "compareAndSet",
            "(ZZ)Z",
            native_atomic_boolean_compare_and_set,
        ),
        ("getAndSet", "(Z)Z", native_atomic_boolean_get_and_set),
        (
            "toString",
            "()Ljava/lang/String;",
            native_atomic_boolean_to_string,
        ),
    ] {
        registry.natives_mut().register(
            "java/util/concurrent/atomic/AtomicBoolean",
            method,
            descriptor,
            handler,
        );
    }
}

/// Registers the synthetic `java.util.concurrent.ConcurrentHashMap` surface.
///
/// Duke stores entries in the same flat field layout used by synthetic
/// `HashMap`: `fields[0]` is the size, followed by interleaved key/value
/// slots. Each instance also receives a coarse host-side mutex in its heap
/// payload. That is intentionally simpler than `HotSpot`'s striped bins, but it
/// gives linearizable Duke-visible mutations because the threading runtime
/// already runs bytecode and callbacks under the shared VM heap lock.
///
/// The `keySet`, `values`, and `entrySet` methods return snapshot copies,
/// matching Duke's synthetic `HashMap` views rather than `HotSpot`'s live views.
#[allow(clippy::too_many_lines)]
fn register_concurrent_hashmap_stdlib(registry: &mut ClassRegistry) {
    let concurrent_map_ctx = ClassContext {
        class_name: "java/util/concurrent/ConcurrentMap".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/util/Map".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(concurrent_map_ctx);

    let concurrent_hashmap_ctx = ClassContext {
        class_name: "java/util/concurrent/ConcurrentHashMap".to_string(),
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
            "java/util/concurrent/ConcurrentMap".to_string(),
            "java/util/Map".to_string(),
        ],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(concurrent_hashmap_ctx);

    for desc in ["()V", "(I)V", "(IF)V", "(IFI)V"] {
        registry.natives_mut().register(
            "java/util/concurrent/ConcurrentHashMap",
            "<init>",
            desc,
            native_concurrent_hashmap_init,
        );
    }
    registry.natives_mut().register(
        "java/util/concurrent/ConcurrentHashMap",
        "<init>",
        "(Ljava/util/Map;)V",
        native_concurrent_hashmap_init_map,
    );

    for (method, descriptor, handler) in [
        (
            "get",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            native_concurrent_hashmap_get as NativeHandler,
        ),
        (
            "getOrDefault",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            native_concurrent_hashmap_get_or_default,
        ),
        (
            "containsKey",
            "(Ljava/lang/Object;)Z",
            native_concurrent_hashmap_contains_key,
        ),
        (
            "containsValue",
            "(Ljava/lang/Object;)Z",
            native_concurrent_hashmap_contains_value,
        ),
        ("size", "()I", native_concurrent_hashmap_size),
        ("isEmpty", "()Z", native_concurrent_hashmap_is_empty),
        (
            "put",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            native_concurrent_hashmap_put,
        ),
        (
            "putIfAbsent",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            native_concurrent_hashmap_put_if_absent,
        ),
        (
            "remove",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            native_concurrent_hashmap_remove,
        ),
        (
            "remove",
            "(Ljava/lang/Object;Ljava/lang/Object;)Z",
            native_concurrent_hashmap_remove_key_value,
        ),
        (
            "replace",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            native_concurrent_hashmap_replace,
        ),
        (
            "replace",
            "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)Z",
            native_concurrent_hashmap_replace_key_value,
        ),
        ("clear", "()V", native_concurrent_hashmap_clear),
        (
            "putAll",
            "(Ljava/util/Map;)V",
            native_concurrent_hashmap_put_all,
        ),
        (
            "keySet",
            "()Ljava/util/Set;",
            native_concurrent_hashmap_key_set,
        ),
        (
            "keySet",
            "()Ljava/util/concurrent/ConcurrentHashMap$KeySetView;",
            native_concurrent_hashmap_key_set,
        ),
        (
            "values",
            "()Ljava/util/Collection;",
            native_concurrent_hashmap_values,
        ),
        (
            "entrySet",
            "()Ljava/util/Set;",
            native_concurrent_hashmap_entry_set,
        ),
    ] {
        registry.natives_mut().register(
            "java/util/concurrent/ConcurrentHashMap",
            method,
            descriptor,
            handler,
        );
    }

    registry.natives_mut().register_callback(
        "java/util/concurrent/ConcurrentHashMap",
        "computeIfAbsent",
        "(Ljava/lang/Object;Ljava/util/function/Function;)Ljava/lang/Object;",
        native_concurrent_hashmap_compute_if_absent,
    );
    registry.natives_mut().register_callback(
        "java/util/concurrent/ConcurrentHashMap",
        "computeIfPresent",
        "(Ljava/lang/Object;Ljava/util/function/BiFunction;)Ljava/lang/Object;",
        native_concurrent_hashmap_compute_if_present,
    );
    registry.natives_mut().register_callback(
        "java/util/concurrent/ConcurrentHashMap",
        "compute",
        "(Ljava/lang/Object;Ljava/util/function/BiFunction;)Ljava/lang/Object;",
        native_concurrent_hashmap_compute,
    );
    registry.natives_mut().register_callback(
        "java/util/concurrent/ConcurrentHashMap",
        "merge",
        "(Ljava/lang/Object;Ljava/lang/Object;Ljava/util/function/BiFunction;)Ljava/lang/Object;",
        native_concurrent_hashmap_merge,
    );
    registry.natives_mut().register_callback(
        "java/util/concurrent/ConcurrentHashMap",
        "forEach",
        "(Ljava/util/function/BiConsumer;)V",
        native_concurrent_hashmap_for_each,
    );
}

/// Registers a synthetic `jdk/internal/misc/Unsafe` surface sufficient to run
/// real `java.util.concurrent` bytecode (notably `ConcurrentHashMap`) under
/// `DUKE_REAL_JDK=1`.
///
/// `Unsafe` is kept fully synthetic (see `KEEP_SYNTHETIC` in `registry.rs`) so
/// the real `Unsafe.<clinit>` — which reaches for `registerNatives` and
/// `UnsafeConstants` — never runs. `getUnsafe()` hands back a stateless
/// synthetic instance; every field/array accessor treats the `long` offset as a
/// positional index into the flat `HeapObject::fields` vector (see the natives
/// in `native.rs` for the offset-encoding contract). This is intentionally
/// simpler than `HotSpot`'s real byte-addressed memory model but is
/// self-consistent because Duke's field and array access are both positional.
fn register_unsafe_stdlib(registry: &mut ClassRegistry) {
    registry.register(ClassContext {
        class_name: "jdk/internal/misc/Unsafe".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    registry.natives_mut().register(
        "jdk/internal/misc/Unsafe",
        "getUnsafe",
        "()Ljdk/internal/misc/Unsafe;",
        native_unsafe_get_unsafe,
    );
    registry.natives_mut().register_callback(
        "jdk/internal/misc/Unsafe",
        "objectFieldOffset",
        "(Ljava/lang/Class;Ljava/lang/String;)J",
        native_unsafe_object_field_offset,
    );

    for (method, descriptor, handler) in [
        (
            "arrayBaseOffset",
            "(Ljava/lang/Class;)I",
            native_unsafe_array_base_offset as NativeHandler,
        ),
        (
            "arrayIndexScale",
            "(Ljava/lang/Class;)I",
            native_unsafe_array_index_scale,
        ),
        (
            "compareAndSetReference",
            "(Ljava/lang/Object;JLjava/lang/Object;Ljava/lang/Object;)Z",
            native_unsafe_compare_and_set_reference,
        ),
        (
            "compareAndSetInt",
            "(Ljava/lang/Object;JII)Z",
            native_unsafe_compare_and_set_int,
        ),
        (
            "compareAndSetLong",
            "(Ljava/lang/Object;JJJ)Z",
            native_unsafe_compare_and_set_long,
        ),
        (
            "getReferenceAcquire",
            "(Ljava/lang/Object;J)Ljava/lang/Object;",
            native_unsafe_get_reference,
        ),
        (
            "putReferenceRelease",
            "(Ljava/lang/Object;JLjava/lang/Object;)V",
            native_unsafe_put_reference,
        ),
        (
            "getAndAddInt",
            "(Ljava/lang/Object;JI)I",
            native_unsafe_get_and_add_int,
        ),
    ] {
        registry
            .natives_mut()
            .register("jdk/internal/misc/Unsafe", method, descriptor, handler);
    }
}

/// Registers the native handler for `jdk/internal/reflect/Reflection`.
///
/// Only the `getCallerClass()` native is provided; the `Reflection` class itself
/// is loaded from the real JDK under `DUKE_REAL_JDK=1` (no synthetic context is
/// registered, so flag-off behavior is unchanged — the entry is simply unused).
/// `getCallerClass` is `@CallerSensitive` machinery reached from
/// `java/util/ServiceLoader.load(Ljava/lang/Class;)`; see
/// `native_reflection_get_caller_class` for the frame-walking semantics.
fn register_reflection_stdlib(registry: &mut ClassRegistry) {
    registry.natives_mut().register(
        "jdk/internal/reflect/Reflection",
        "getCallerClass",
        "()Ljava/lang/Class;",
        native_reflection_get_caller_class,
    );
}

/// Registers the native handler for `jdk/internal/misc/VM`.
///
/// Only the `initialize()V` native is provided; the `VM` class itself is loaded
/// from the real JDK under `DUKE_REAL_JDK=1` (no synthetic context is registered,
/// so flag-off behavior is unchanged — the entry is simply unused). `initialize`
/// is reached from `VM.<clinit>`, itself triggered by `VM.isBooted()` in
/// `java/util/ServiceLoader.<init>`; see `native_vm_initialize` for how it seeds
/// the booted init level.
fn register_vm_stdlib(registry: &mut ClassRegistry) {
    registry.natives_mut().register_callback(
        "jdk/internal/misc/VM",
        "initialize",
        "()V",
        native_vm_initialize,
    );
}

fn lock_interface_context(name: &str) -> ClassContext {
    ClassContext {
        class_name: name.to_string(),
        super_class: None,
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    }
}

/// Registers synthetic `java.util.concurrent.locks` classes.
///
/// The actual lock state lives in host payloads attached to the heap objects;
/// these synthetic contexts provide the Java type names and native dispatch
/// surface required by javac-compiled fixtures.
#[allow(clippy::too_many_lines)]
fn register_locks_stdlib(registry: &mut ClassRegistry) {
    for interface_name in [
        "java/util/concurrent/locks/Lock",
        "java/util/concurrent/locks/Condition",
        "java/util/concurrent/locks/ReadWriteLock",
    ] {
        registry.register(lock_interface_context(interface_name));
    }

    registry.register(ClassContext {
        class_name: "java/util/concurrent/locks/ReentrantLock".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/util/concurrent/locks/Lock".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    registry.register(ClassContext {
        class_name: "duke/util/concurrent/ConditionObject".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/util/concurrent/locks/Condition".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    registry.register(ClassContext {
        class_name: "java/util/concurrent/locks/ReentrantReadWriteLock".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            synthetic_field(
                "readerLock",
                "Ljava/util/concurrent/locks/ReentrantReadWriteLock$ReadLock;",
                false,
            ),
            synthetic_field(
                "writerLock",
                "Ljava/util/concurrent/locks/ReentrantReadWriteLock$WriteLock;",
                false,
            ),
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/concurrent/locks/ReadWriteLock".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    registry.register(ClassContext {
        class_name: "java/util/concurrent/locks/ReentrantReadWriteLock$ReadLock".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/util/concurrent/locks/Lock".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    registry.register(ClassContext {
        class_name: "java/util/concurrent/locks/ReentrantReadWriteLock$WriteLock".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/util/concurrent/locks/Lock".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    for (method, descriptor, handler) in [
        ("<init>", "()V", native_reentrant_lock_init as NativeHandler),
        ("<init>", "(Z)V", native_reentrant_lock_init_fair),
        ("lock", "()V", native_reentrant_lock_lock),
        ("lockInterruptibly", "()V", native_reentrant_lock_lock),
        ("tryLock", "()Z", native_reentrant_lock_try_lock),
        ("unlock", "()V", native_reentrant_lock_unlock),
        (
            "newCondition",
            "()Ljava/util/concurrent/locks/Condition;",
            native_reentrant_lock_new_condition,
        ),
        ("getHoldCount", "()I", native_reentrant_lock_get_hold_count),
        (
            "isHeldByCurrentThread",
            "()Z",
            native_reentrant_lock_is_held_by_current_thread,
        ),
        ("isLocked", "()Z", native_reentrant_lock_is_locked),
        ("isFair", "()Z", native_reentrant_lock_is_fair),
    ] {
        registry.natives_mut().register(
            "java/util/concurrent/locks/ReentrantLock",
            method,
            descriptor,
            handler,
        );
    }

    for (method, descriptor, handler) in [
        ("await", "()V", native_condition_await as NativeHandler),
        ("awaitUninterruptibly", "()V", native_condition_await),
        ("awaitNanos", "(J)J", native_condition_await_nanos),
        ("signal", "()V", native_condition_signal),
        ("signalAll", "()V", native_condition_signal_all),
    ] {
        registry.natives_mut().register(
            "duke/util/concurrent/ConditionObject",
            method,
            descriptor,
            handler,
        );
    }

    for desc in ["()V", "(Z)V"] {
        registry.natives_mut().register(
            "java/util/concurrent/locks/ReentrantReadWriteLock",
            "<init>",
            desc,
            native_reentrant_read_write_lock_init,
        );
    }
    for desc in [
        "()Ljava/util/concurrent/locks/ReentrantReadWriteLock$ReadLock;",
        "()Ljava/util/concurrent/locks/Lock;",
    ] {
        registry.natives_mut().register(
            "java/util/concurrent/locks/ReentrantReadWriteLock",
            "readLock",
            desc,
            native_reentrant_read_write_lock_read_lock,
        );
    }
    for desc in [
        "()Ljava/util/concurrent/locks/ReentrantReadWriteLock$WriteLock;",
        "()Ljava/util/concurrent/locks/Lock;",
    ] {
        registry.natives_mut().register(
            "java/util/concurrent/locks/ReentrantReadWriteLock",
            "writeLock",
            desc,
            native_reentrant_read_write_lock_write_lock,
        );
    }
    for (method, descriptor, handler) in [
        (
            "isWriteLocked",
            "()Z",
            native_reentrant_read_write_lock_is_write_locked as NativeHandler,
        ),
        (
            "isWriteLockedByCurrentThread",
            "()Z",
            native_reentrant_read_write_lock_is_write_locked_by_current_thread,
        ),
        (
            "getWriteHoldCount",
            "()I",
            native_reentrant_read_write_lock_get_write_hold_count,
        ),
        (
            "getReadHoldCount",
            "()I",
            native_reentrant_read_write_lock_get_read_hold_count,
        ),
        (
            "getReadLockCount",
            "()I",
            native_reentrant_read_write_lock_get_read_lock_count,
        ),
    ] {
        registry.natives_mut().register(
            "java/util/concurrent/locks/ReentrantReadWriteLock",
            method,
            descriptor,
            handler,
        );
    }

    for (class_name, lock_handler, try_handler, unlock_handler, condition_handler) in [
        (
            "java/util/concurrent/locks/ReentrantReadWriteLock$ReadLock",
            native_read_lock_lock as NativeHandler,
            native_read_lock_try_lock as NativeHandler,
            native_read_lock_unlock as NativeHandler,
            native_read_lock_new_condition as NativeHandler,
        ),
        (
            "java/util/concurrent/locks/ReentrantReadWriteLock$WriteLock",
            native_write_lock_lock as NativeHandler,
            native_write_lock_try_lock as NativeHandler,
            native_write_lock_unlock as NativeHandler,
            native_write_lock_new_condition as NativeHandler,
        ),
    ] {
        registry
            .natives_mut()
            .register(class_name, "lock", "()V", lock_handler);
        registry
            .natives_mut()
            .register(class_name, "lockInterruptibly", "()V", lock_handler);
        registry
            .natives_mut()
            .register(class_name, "tryLock", "()Z", try_handler);
        registry
            .natives_mut()
            .register(class_name, "unlock", "()V", unlock_handler);
        registry.natives_mut().register(
            class_name,
            "newCondition",
            "()Ljava/util/concurrent/locks/Condition;",
            condition_handler,
        );
    }
}

/// Registers synthetic `java.util.concurrent` synchronization primitives.
///
/// These are leaf implementations backed by host-side payload state rather than
/// a public guest-visible `AbstractQueuedSynchronizer` surface. Java permits may
/// grow through unmatched `Semaphore.release` calls, matching the JDK contract.
#[allow(clippy::too_many_lines)]
fn register_sync_primitives_stdlib(registry: &mut ClassRegistry) {
    registry.register(empty_synthetic_context(
        "java/util/concurrent/CountDownLatch",
        "java/lang/Object",
    ));
    registry.register(empty_synthetic_context(
        "java/util/concurrent/Semaphore",
        "java/lang/Object",
    ));
    registry.register(ClassContext {
        class_name: "java/util/concurrent/CyclicBarrier".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![synthetic_field(
            "barrierAction",
            "Ljava/lang/Runnable;",
            false,
        )],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    for (method, descriptor, handler) in [
        (
            "<init>",
            "(I)V",
            native_count_down_latch_init as NativeHandler,
        ),
        ("await", "()V", native_count_down_latch_await),
        (
            "await",
            "(JLjava/util/concurrent/TimeUnit;)Z",
            native_count_down_latch_await_timeout,
        ),
        ("countDown", "()V", native_count_down_latch_count_down),
        ("getCount", "()J", native_count_down_latch_get_count),
        (
            "toString",
            "()Ljava/lang/String;",
            native_count_down_latch_to_string,
        ),
    ] {
        registry.natives_mut().register(
            "java/util/concurrent/CountDownLatch",
            method,
            descriptor,
            handler,
        );
    }

    for (method, descriptor, handler) in [
        ("<init>", "(I)V", native_semaphore_init as NativeHandler),
        ("<init>", "(IZ)V", native_semaphore_init_fair),
        ("acquire", "()V", native_semaphore_acquire),
        ("acquire", "(I)V", native_semaphore_acquire_many),
        (
            "acquireUninterruptibly",
            "()V",
            native_semaphore_acquire_uninterruptibly,
        ),
        (
            "acquireUninterruptibly",
            "(I)V",
            native_semaphore_acquire_uninterruptibly_many,
        ),
        ("tryAcquire", "()Z", native_semaphore_try_acquire),
        ("tryAcquire", "(I)Z", native_semaphore_try_acquire_many),
        (
            "tryAcquire",
            "(JLjava/util/concurrent/TimeUnit;)Z",
            native_semaphore_try_acquire_timeout,
        ),
        ("release", "()V", native_semaphore_release),
        ("release", "(I)V", native_semaphore_release_many),
        (
            "availablePermits",
            "()I",
            native_semaphore_available_permits,
        ),
        ("drainPermits", "()I", native_semaphore_drain_permits),
        (
            "hasQueuedThreads",
            "()Z",
            native_semaphore_has_queued_threads,
        ),
        ("getQueueLength", "()I", native_semaphore_get_queue_length),
        ("isFair", "()Z", native_semaphore_is_fair),
        (
            "toString",
            "()Ljava/lang/String;",
            native_semaphore_to_string,
        ),
    ] {
        registry.natives_mut().register(
            "java/util/concurrent/Semaphore",
            method,
            descriptor,
            handler,
        );
    }

    for (method, descriptor, handler) in [
        (
            "<init>",
            "(I)V",
            native_cyclic_barrier_init as NativeHandler,
        ),
        (
            "<init>",
            "(ILjava/lang/Runnable;)V",
            native_cyclic_barrier_init_action,
        ),
        ("getParties", "()I", native_cyclic_barrier_get_parties),
        (
            "getNumberWaiting",
            "()I",
            native_cyclic_barrier_get_number_waiting,
        ),
        ("isBroken", "()Z", native_cyclic_barrier_is_broken),
        ("reset", "()V", native_cyclic_barrier_reset),
        (
            "toString",
            "()Ljava/lang/String;",
            native_cyclic_barrier_to_string,
        ),
    ] {
        registry.natives_mut().register(
            "java/util/concurrent/CyclicBarrier",
            method,
            descriptor,
            handler,
        );
    }
    registry.natives_mut().register_callback(
        "java/util/concurrent/CyclicBarrier",
        "await",
        "()I",
        native_cyclic_barrier_await,
    );
    registry.natives_mut().register_callback(
        "java/util/concurrent/CyclicBarrier",
        "await",
        "(JLjava/util/concurrent/TimeUnit;)I",
        native_cyclic_barrier_await_timeout,
    );
}

fn allocate_time_unit(heap: &mut duke_gc::Heap, name: &str, ordinal: i32, nanos: i64) -> Slot {
    let unit_ref = heap.allocate("java/util/concurrent/TimeUnit".to_string(), 3);
    let name_ref = heap.allocate_string(name.to_string());
    if let Ok(unit) = heap.get_mut(unit_ref) {
        unit.fields[0] = Slot::Reference(Some(name_ref));
        unit.fields[1] = Slot::Int(ordinal);
        unit.fields[2] = Slot::Long(nanos);
    }
    Slot::Reference(Some(unit_ref))
}

/// Registers Duke's minimal `ExecutorService` / `Future` surface.
///
/// The Java-visible types are synthetic, while queueing and worker state live in
/// host payloads attached to `duke/util/concurrent/DukeExecutorService`.
#[allow(clippy::too_many_lines)]
fn register_executor_stdlib(registry: &mut ClassRegistry, heap: &mut duke_gc::Heap) {
    for interface_name in [
        "java/util/concurrent/Executor",
        "java/util/concurrent/ExecutorService",
        "java/util/concurrent/Future",
        "java/util/concurrent/Callable",
    ] {
        registry.register(lock_interface_context(interface_name));
    }

    registry.register(empty_synthetic_context(
        "java/util/concurrent/Executors",
        "java/lang/Object",
    ));

    registry.register(ClassContext {
        class_name: "duke/util/concurrent/DukeExecutorService".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            synthetic_field("shutdown", "Z", false),
            synthetic_field("awaitDeadlineNanos", "J", false),
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec![
            "java/util/concurrent/ExecutorService".to_string(),
            "java/util/concurrent/Executor".to_string(),
        ],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    registry.register(ClassContext {
        class_name: "duke/util/concurrent/DukeFuture".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            synthetic_field("state", "I", false),
            synthetic_field("result", "Ljava/lang/Object;", false),
            synthetic_field("exception", "Ljava/lang/Throwable;", false),
            synthetic_field("waitDeadlineNanos", "J", false),
            synthetic_field("task", "Ljava/lang/Object;", false),
        ],
        static_fields: Vec::new(),
        instance_field_count: 5,
        interfaces: vec!["java/util/concurrent/Future".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    let time_units = [
        ("NANOSECONDS", 0, 1_i64),
        ("MICROSECONDS", 1, 1_000),
        ("MILLISECONDS", 2, 1_000_000),
        ("SECONDS", 3, 1_000_000_000),
        ("MINUTES", 4, 60_000_000_000),
        ("HOURS", 5, 3_600_000_000_000),
        ("DAYS", 6, 86_400_000_000_000),
    ];
    let mut time_unit_fields: Vec<FieldEntry> = time_units
        .iter()
        .map(|(name, _, _)| synthetic_field(name, "Ljava/util/concurrent/TimeUnit;", true))
        .collect();
    time_unit_fields.push(synthetic_field("nanosPerUnit", "J", false));
    let time_unit_static_fields: Vec<Slot> = time_units
        .iter()
        .map(|(name, ordinal, nanos)| allocate_time_unit(heap, name, *ordinal, *nanos))
        .collect();
    registry.register(ClassContext {
        class_name: "java/util/concurrent/TimeUnit".to_string(),
        super_class: Some("java/lang/Enum".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: time_unit_fields,
        static_fields: time_unit_static_fields,
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    });

    for (method, descriptor, handler) in [
        (
            "newFixedThreadPool",
            "(I)Ljava/util/concurrent/ExecutorService;",
            native_executors_new_fixed_thread_pool as NativeHandler,
        ),
        (
            "newSingleThreadExecutor",
            "()Ljava/util/concurrent/ExecutorService;",
            native_executors_new_single_thread_executor,
        ),
        (
            "newCachedThreadPool",
            "()Ljava/util/concurrent/ExecutorService;",
            native_executors_new_cached_thread_pool,
        ),
    ] {
        registry.natives_mut().register(
            "java/util/concurrent/Executors",
            method,
            descriptor,
            handler,
        );
    }

    for class_name in [
        "duke/util/concurrent/DukeExecutorService",
        "java/util/concurrent/ExecutorService",
    ] {
        registry.natives_mut().register_callback(
            class_name,
            "submit",
            "(Ljava/lang/Runnable;)Ljava/util/concurrent/Future;",
            native_executor_submit_runnable,
        );
        registry.natives_mut().register_callback(
            class_name,
            "submit",
            "(Ljava/lang/Runnable;Ljava/lang/Object;)Ljava/util/concurrent/Future;",
            native_executor_submit_runnable_result,
        );
        registry.natives_mut().register_callback(
            class_name,
            "submit",
            "(Ljava/util/concurrent/Callable;)Ljava/util/concurrent/Future;",
            native_executor_submit_callable,
        );
        registry
            .natives_mut()
            .register(class_name, "shutdown", "()V", native_executor_shutdown);
        registry.natives_mut().register(
            class_name,
            "awaitTermination",
            "(JLjava/util/concurrent/TimeUnit;)Z",
            native_executor_await_termination,
        );
        registry.natives_mut().register(
            class_name,
            "isShutdown",
            "()Z",
            native_executor_is_shutdown,
        );
        registry.natives_mut().register(
            class_name,
            "isTerminated",
            "()Z",
            native_executor_is_terminated,
        );
    }
    for class_name in [
        "duke/util/concurrent/DukeExecutorService",
        "java/util/concurrent/Executor",
    ] {
        registry.natives_mut().register_callback(
            class_name,
            "execute",
            "(Ljava/lang/Runnable;)V",
            native_executor_execute,
        );
    }

    for class_name in [
        "duke/util/concurrent/DukeFuture",
        "java/util/concurrent/Future",
    ] {
        registry.natives_mut().register(
            class_name,
            "get",
            "()Ljava/lang/Object;",
            native_future_get,
        );
        registry.natives_mut().register(
            class_name,
            "get",
            "(JLjava/util/concurrent/TimeUnit;)Ljava/lang/Object;",
            native_future_get_timeout,
        );
        registry
            .natives_mut()
            .register(class_name, "cancel", "(Z)Z", native_future_cancel);
        registry.natives_mut().register(
            class_name,
            "isCancelled",
            "()Z",
            native_future_is_cancelled,
        );
        registry
            .natives_mut()
            .register(class_name, "isDone", "()Z", native_future_is_done);
    }

    registry.natives_mut().register(
        "java/util/concurrent/TimeUnit",
        "toMillis",
        "(J)J",
        native_timeunit_to_millis,
    );
    registry.natives_mut().register(
        "java/util/concurrent/TimeUnit",
        "toNanos",
        "(J)J",
        native_timeunit_to_nanos,
    );

    for (name, super_name) in [
        (
            "java/util/concurrent/ExecutionException",
            "java/lang/Exception",
        ),
        (
            "java/util/concurrent/TimeoutException",
            "java/lang/Exception",
        ),
        (
            "java/util/concurrent/BrokenBarrierException",
            "java/lang/Exception",
        ),
        (
            "java/util/concurrent/CancellationException",
            "java/lang/IllegalStateException",
        ),
        (
            "java/util/concurrent/RejectedExecutionException",
            "java/lang/RuntimeException",
        ),
    ] {
        registry.register(ClassContext {
            class_name: name.to_string(),
            super_class: Some(super_name.to_string()),
            constant_pool: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            static_fields: Vec::new(),
            instance_field_count: 0,
            interfaces: Vec::new(),
            bootstrap_methods: Vec::new(),
            load_source: ClassLoadSource::Synthetic,
        });
        registry
            .natives_mut()
            .register(name, "<init>", "()V", native_throwable_init);
        registry.natives_mut().register(
            name,
            "<init>",
            "(Ljava/lang/String;)V",
            native_throwable_init_string,
        );
        registry.natives_mut().register(
            name,
            "<init>",
            "(Ljava/lang/String;Ljava/lang/Throwable;)V",
            native_throwable_init_string_cause,
        );
        registry.natives_mut().register(
            name,
            "<init>",
            "(Ljava/lang/Throwable;)V",
            native_throwable_init_cause,
        );
    }
}

#[allow(clippy::too_many_lines)]
/// Bootstraps the minimal JDK standard library classes needed for native method support.
///
/// This function synthesises minimal core classes (like `java/lang/System`, `java/io/PrintStream`, etc.)
/// and registers their native method handlers. This allows the JVM to execute basic Java code without
/// requiring a full `rt.jar` to be loaded. It configures the essential runtime environment.
///
/// # Examples
///
/// ```
/// use duke_interpreter::ClassRegistry;
/// use duke_interpreter::bootstrap_stdlib;
/// use duke_gc::Heap;
///
/// let mut registry = ClassRegistry::new();
/// let mut heap = Heap::new();
///
/// // Populate the registry and heap with standard classes and natives.
/// bootstrap_stdlib(&mut registry, &mut heap);
///
/// // Now the JVM can resolve java/lang/System and invoke println.
/// assert!(registry.contains("java/lang/System"));
/// ```
pub fn bootstrap_stdlib(registry: &mut ClassRegistry, heap: &mut duke_gc::Heap) {
    // Allocate PrintStream objects for System.out and System.err.
    let ps_out_ref = heap.allocate("java/io/PrintStream".to_string(), 1);
    let ps_err_ref = heap.allocate("java/io/PrintStream".to_string(), 1);

    // Create java/lang/System ClassContext with stream static fields `out`, `err`, `in`.
    //
    // The stream slots start null and are seeded immediately below through the
    // shared `set_system_stream` helper — the SAME store path the setOut0/setErr0
    // natives use — so the System-init seeding code runs live on every startup
    // rather than being a dead future-only code path. `System` itself stays on
    // `KEEP_SYNTHETIC` this wave; see the setOut0/setErr0/setIn0 block below.
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
            FieldEntry {
                name: "in".to_string(),
                descriptor: "Ljava/io/InputStream;".to_string(),
                is_static: true,
            },
        ],
        static_fields: vec![
            Slot::Reference(None),
            Slot::Reference(None),
            Slot::Reference(None),
        ],
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(system_ctx);

    // Seed System.out/err through the shared setOut0/setErr0 store path so the two
    // heap-allocated PrintStreams land in the same `out`/`err` static slots as
    // before (byte-identical). `in` intentionally stays null: nothing reads it
    // today, and `setIn0` exists for the future real-layout migration wave.
    seed_system_streams(registry, ps_out_ref, ps_err_ref);
    register_system_stream_natives(registry);

    // Create java/io/PrintStream ClassContext (empty — all methods are native).
    let ps_ctx = ClassContext {
        class_name: "java/io/PrintStream".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "out".to_string(),
            descriptor: "Ljava/io/OutputStream;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
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
    registry.natives_mut().register(
        "java/io/PrintStream",
        "<init>",
        "(Ljava/io/OutputStream;)V",
        native_printstream_init_output_stream,
    );

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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(input_stream_ctx);
    let resource_input_stream_ctx = ClassContext {
        class_name: "duke/io/ResourceInputStream".to_string(),
        super_class: Some("java/io/InputStream".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "bytes".to_string(),
                descriptor: "[B".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "cursor".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "closed".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 3,
        interfaces: vec!["java/lang/AutoCloseable".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(resource_input_stream_ctx);
    for (method, descriptor, handler) in [
        (
            "read",
            "()I",
            native_resource_input_stream_read as NativeHandler,
        ),
        (
            "read",
            "([B)I",
            native_resource_input_stream_read_bytes as NativeHandler,
        ),
        (
            "read",
            "([BII)I",
            native_resource_input_stream_read_bytes_slice as NativeHandler,
        ),
        (
            "available",
            "()I",
            native_resource_input_stream_available as NativeHandler,
        ),
        (
            "skip",
            "(J)J",
            native_resource_input_stream_skip as NativeHandler,
        ),
        (
            "close",
            "()V",
            native_resource_input_stream_close as NativeHandler,
        ),
    ] {
        registry
            .natives_mut()
            .register("duke/io/ResourceInputStream", method, descriptor, handler);
    }

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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(output_stream_ctx);

    let byte_array_output_stream_ctx = ClassContext {
        class_name: "java/io/ByteArrayOutputStream".to_string(),
        super_class: Some("java/io/OutputStream".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(byte_array_output_stream_ctx);
    registry.natives_mut().register(
        "java/io/ByteArrayOutputStream",
        "<init>",
        "()V",
        native_byte_array_output_stream_init,
    );
    registry.natives_mut().register(
        "java/io/ByteArrayOutputStream",
        "toString",
        "()Ljava/lang/String;",
        native_byte_array_output_stream_to_string,
    );

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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        "availableProcessors",
        "()I",
        native_runtime_available_processors,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
    registry.natives_mut().register(
        "java/lang/String",
        "equalsIgnoreCase",
        "(Ljava/lang/String;)Z",
        native_string_equalsignorecase,
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
        "indexOf",
        "(Ljava/lang/String;I)I",
        native_string_indexof_from,
    );
    registry.natives_mut().register(
        "java/lang/String",
        "lastIndexOf",
        "(Ljava/lang/String;I)I",
        native_string_last_indexof_from,
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
        "split",
        "(Ljava/lang/String;I)[Ljava/lang/String;",
        native_string_split_limit,
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
    // PrintStream.flush — Duke writes directly, so flushing is a no-op.
    registry
        .natives_mut()
        .register("java/io/PrintStream", "flush", "()V", native_void_noop);

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
        "getSecurityManager",
        "()Ljava/lang/SecurityManager;",
        native_system_get_security_manager,
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
        load_source: ClassLoadSource::Synthetic,
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

    register_charset_stdlib(registry, heap);
    register_base64_stdlib(registry);
    register_atomic_stdlib(registry);
    register_concurrent_hashmap_stdlib(registry);
    register_unsafe_stdlib(registry);
    register_reflection_stdlib(registry);
    register_vm_stdlib(registry);
    register_locks_stdlib(registry);
    register_sync_primitives_stdlib(registry);
    register_executor_stdlib(registry, heap);
    register_jul_stdlib(registry, heap);

    // java/lang/Class — lightweight stub for class literals
    let class_ctx = ClassContext {
        class_name: "java/lang/Class".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/lang/reflect/Type".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
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
    registry
        .natives_mut()
        .register("java/lang/Class", "isArray", "()Z", native_class_is_array);
    registry.natives_mut().register(
        "java/lang/Class",
        "getPrimitiveClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        native_class_get_primitive_class,
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
        "getAnnotations",
        "()[Ljava/lang/annotation/Annotation;",
        native_class_get_annotations,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getDeclaredAnnotations",
        "()[Ljava/lang/annotation/Annotation;",
        native_class_get_declared_annotations,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getAnnotation",
        "(Ljava/lang/Class;)Ljava/lang/annotation/Annotation;",
        native_class_get_annotation,
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
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getResourceAsStream",
        "(Ljava/lang/String;)Ljava/io/InputStream;",
        native_class_get_resource_as_stream,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getResource",
        "(Ljava/lang/String;)Ljava/net/URL;",
        native_class_get_resource,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(url_ctx);
    registry.natives_mut().register(
        "java/net/URL",
        "<init>",
        "(Ljava/lang/String;)V",
        native_url_init,
    );
    registry.natives_mut().register(
        "java/net/URL",
        "toString",
        "()Ljava/lang/String;",
        native_url_to_string,
    );
    registry.natives_mut().register(
        "java/net/URL",
        "toExternalForm",
        "()Ljava/lang/String;",
        native_url_to_external_form,
    );
    registry.natives_mut().register(
        "java/net/URL",
        "getPath",
        "()Ljava/lang/String;",
        native_url_get_path,
    );
    registry.natives_mut().register(
        "java/net/URL",
        "openStream",
        "()Ljava/io/InputStream;",
        native_url_open_stream,
    );
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

    let url_class_path_ctx = ClassContext {
        class_name: "jdk/internal/loader/URLClassPath".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "path".to_string(),
            descriptor: "Ljava/util/ArrayList;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(url_class_path_ctx);

    let url_class_loader_ctx = ClassContext {
        class_name: "java/net/URLClassLoader".to_string(),
        super_class: Some("java/lang/ClassLoader".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "ucp".to_string(),
            descriptor: "Ljdk/internal/loader/URLClassPath;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(url_class_loader_ctx);
    registry.natives_mut().register(
        "java/net/URLClassLoader",
        "<init>",
        "([Ljava/net/URL;)V",
        native_url_class_loader_init,
    );
    registry.natives_mut().register(
        "java/net/URLClassLoader",
        "<init>",
        "([Ljava/net/URL;Ljava/lang/ClassLoader;)V",
        native_url_class_loader_init,
    );
    registry.natives_mut().register_callback(
        "java/net/URLClassLoader",
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        native_url_class_loader_load_class,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
    registry.natives_mut().register_callback(
        "java/lang/reflect/Method",
        "getAnnotations",
        "()[Ljava/lang/annotation/Annotation;",
        native_reflect_method_get_annotations,
    );
    registry.natives_mut().register_callback(
        "java/lang/reflect/Method",
        "getDeclaredAnnotations",
        "()[Ljava/lang/annotation/Annotation;",
        native_reflect_method_get_declared_annotations,
    );
    registry.natives_mut().register_callback(
        "java/lang/reflect/Method",
        "getAnnotation",
        "(Ljava/lang/Class;)Ljava/lang/annotation/Annotation;",
        native_reflect_method_get_annotation,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
    registry.natives_mut().register_callback(
        "java/lang/reflect/Field",
        "getAnnotations",
        "()[Ljava/lang/annotation/Annotation;",
        native_reflect_field_get_annotations,
    );
    registry.natives_mut().register_callback(
        "java/lang/reflect/Field",
        "getDeclaredAnnotations",
        "()[Ljava/lang/annotation/Annotation;",
        native_reflect_field_get_declared_annotations,
    );
    registry.natives_mut().register_callback(
        "java/lang/reflect/Field",
        "getAnnotation",
        "(Ljava/lang/Class;)Ljava/lang/annotation/Annotation;",
        native_reflect_field_get_annotation,
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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(runnable_ctx);

    let class_loader_ctx = ClassContext {
        class_name: "java/lang/ClassLoader".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        // Static-only field caching the single system ClassLoader instance
        // returned by `getSystemClassLoader` (see SYSTEM_CLASS_LOADER_FIELD).
        fields: vec![FieldEntry {
            name: SYSTEM_CLASS_LOADER_FIELD.to_string(),
            descriptor: "Ljava/lang/ClassLoader;".to_string(),
            is_static: true,
        }],
        static_fields: vec![Slot::Reference(None)],
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(class_loader_ctx);
    // Real-JDK shadow bootstrap: `java/lang/ClassLoader` is not on KEEP_SYNTHETIC, so
    // under the flag the real JDK-21 ClassLoader classfile loads and its `<clinit>`
    // runs. Its first act is a private static `registerNatives()V` (genuinely `native`
    // in the JDK, no bytecode). Without a native, the interpreter runs the empty body
    // off its end (`FellOffEnd`). This no-op satisfies the hook so `<clinit>` proceeds.
    registry.natives_mut().register(
        "java/lang/ClassLoader",
        "registerNatives",
        "()V",
        native_class_loader_register_natives,
    );
    registry.natives_mut().register(
        "java/lang/ClassLoader",
        "registerAsParallelCapable",
        "()Z",
        native_class_loader_register_as_parallel_capable,
    );
    registry.natives_mut().register_callback(
        "java/lang/ClassLoader",
        "getResourceAsStream",
        "(Ljava/lang/String;)Ljava/io/InputStream;",
        native_class_loader_get_resource_as_stream,
    );
    registry.natives_mut().register_callback(
        "java/lang/ClassLoader",
        "getResource",
        "(Ljava/lang/String;)Ljava/net/URL;",
        native_class_loader_get_resource,
    );
    registry.natives_mut().register_callback(
        "java/lang/ClassLoader",
        "getResources",
        "(Ljava/lang/String;)Ljava/util/Enumeration;",
        native_class_loader_get_resources,
    );
    registry.natives_mut().register_callback(
        "java/lang/ClassLoader",
        "getSystemResourceAsStream",
        "(Ljava/lang/String;)Ljava/io/InputStream;",
        native_class_loader_get_system_resource_as_stream,
    );
    // ┌──────────────────────────────────────────────────────────────────────┐
    // │ System ClassLoader accessors (Spring Boot ladder / app frontier)      │
    // └──────────────────────────────────────────────────────────────────────┘
    registry.natives_mut().register_callback(
        "java/lang/ClassLoader",
        "getSystemResources",
        "(Ljava/lang/String;)Ljava/util/Enumeration;",
        native_class_loader_get_system_resources,
    );
    registry.natives_mut().register_callback(
        "java/lang/ClassLoader",
        "getSystemClassLoader",
        "()Ljava/lang/ClassLoader;",
        native_class_loader_get_system_class_loader,
    );
    // Base `ClassLoader.loadClass` delegates to the parent/default loader first and
    // then the receiver's runtime paths — exactly `native_url_class_loader_load_class`.
    registry.natives_mut().register_callback(
        "java/lang/ClassLoader",
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        native_url_class_loader_load_class,
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
            FieldEntry {
                name: "interrupted".to_string(),
                descriptor: "Z".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "hostKey".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "contextClassLoader".to_string(),
                descriptor: "Ljava/lang/ClassLoader;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 5,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
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
        "interrupt",
        "()V",
        native_thread_interrupt,
    );
    registry.natives_mut().register(
        "java/lang/Thread",
        "isInterrupted",
        "()Z",
        native_thread_is_interrupted,
    );
    registry.natives_mut().register(
        "java/lang/Thread",
        "interrupted",
        "()Z",
        native_thread_interrupted,
    );
    registry.natives_mut().register(
        "java/lang/Thread",
        "getName",
        "()Ljava/lang/String;",
        native_thread_get_name,
    );
    registry.natives_mut().register(
        "java/lang/Thread",
        "getContextClassLoader",
        "()Ljava/lang/ClassLoader;",
        native_thread_get_context_class_loader,
    );
    registry.natives_mut().register(
        "java/lang/Thread",
        "setContextClassLoader",
        "(Ljava/lang/ClassLoader;)V",
        native_thread_set_context_class_loader,
    );

    let stack_trace_element_ctx = ClassContext {
        class_name: "java/lang/StackTraceElement".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "declaringClass".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "methodName".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "fileName".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "lineNumber".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 4,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(stack_trace_element_ctx);
    registry.natives_mut().register(
        "java/lang/StackTraceElement",
        "<init>",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;I)V",
        native_stack_trace_element_init,
    );
    registry.natives_mut().register(
        "java/lang/StackTraceElement",
        "getClassName",
        "()Ljava/lang/String;",
        native_stack_trace_element_get_class_name,
    );
    registry.natives_mut().register(
        "java/lang/StackTraceElement",
        "getMethodName",
        "()Ljava/lang/String;",
        native_stack_trace_element_get_method_name,
    );
    registry.natives_mut().register(
        "java/lang/StackTraceElement",
        "getFileName",
        "()Ljava/lang/String;",
        native_stack_trace_element_get_file_name,
    );
    registry.natives_mut().register(
        "java/lang/StackTraceElement",
        "getLineNumber",
        "()I",
        native_stack_trace_element_get_line_number,
    );
    registry.natives_mut().register(
        "java/lang/StackTraceElement",
        "isNativeMethod",
        "()Z",
        native_stack_trace_element_is_native_method,
    );
    registry.natives_mut().register(
        "java/lang/StackTraceElement",
        "toString",
        "()Ljava/lang/String;",
        native_stack_trace_element_to_string,
    );

    // java/lang/Throwable extends Object
    let throwable_ctx = ClassContext {
        class_name: "java/lang/Throwable".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "cause".to_string(),
                descriptor: "Ljava/lang/Throwable;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "stackTrace".to_string(),
                descriptor: "[Ljava/lang/StackTraceElement;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "suppressedExceptions".to_string(),
                descriptor: "[Ljava/lang/Throwable;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 3,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(throwable_ctx);
    registry.natives_mut().register(
        "java/lang/Throwable",
        "addSuppressed",
        "(Ljava/lang/Throwable;)V",
        native_throwable_add_suppressed,
    );
    registry.natives_mut().register(
        "java/lang/Throwable",
        "<init>",
        "()V",
        native_throwable_init,
    );
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
        "getLocalizedMessage",
        "()Ljava/lang/String;",
        native_throwable_get_message,
    );
    registry.natives_mut().register(
        "java/lang/Throwable",
        "getCause",
        "()Ljava/lang/Throwable;",
        native_throwable_get_cause,
    );
    registry.natives_mut().register(
        "java/lang/Throwable",
        "toString",
        "()Ljava/lang/String;",
        native_throwable_tostring,
    );
    registry.natives_mut().register(
        "java/lang/Throwable",
        "fillInStackTrace",
        "()Ljava/lang/Throwable;",
        native_throwable_fill_in_stack_trace,
    );
    registry.natives_mut().register(
        "java/lang/Throwable",
        "getStackTrace",
        "()[Ljava/lang/StackTraceElement;",
        native_throwable_get_stack_trace,
    );
    registry.natives_mut().register(
        "java/lang/Throwable",
        "setStackTrace",
        "([Ljava/lang/StackTraceElement;)V",
        native_throwable_set_stack_trace,
    );
    registry.natives_mut().register(
        "java/lang/Throwable",
        "getSuppressed",
        "()[Ljava/lang/Throwable;",
        native_throwable_get_suppressed,
    );
    registry.natives_mut().register(
        "java/lang/Throwable",
        "printStackTrace",
        "()V",
        native_throwable_print_stack_trace,
    );
    registry.natives_mut().register(
        "java/lang/Throwable",
        "printStackTrace",
        "(Ljava/io/PrintStream;)V",
        native_throwable_print_stack_trace_print_stream,
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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(exception_ctx);
    registry.natives_mut().register(
        "java/lang/Exception",
        "<init>",
        "()V",
        native_throwable_init,
    );
    registry.natives_mut().register(
        "java/lang/Exception",
        "<init>",
        "(Ljava/lang/String;)V",
        native_throwable_init_string,
    );

    {
        let name = "java/lang/InterruptedException";
        registry.register(ClassContext {
            class_name: name.to_string(),
            super_class: Some("java/lang/Exception".to_string()),
            constant_pool: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            static_fields: Vec::new(),
            instance_field_count: 0,
            interfaces: Vec::new(),
            bootstrap_methods: Vec::new(),
            load_source: ClassLoadSource::Synthetic,
        });
        registry
            .natives_mut()
            .register(name, "<init>", "()V", native_throwable_init);
        registry.natives_mut().register(
            name,
            "<init>",
            "(Ljava/lang/String;)V",
            native_throwable_init_string,
        );
    }

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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(rte_ctx);
    registry.natives_mut().register(
        "java/lang/RuntimeException",
        "<init>",
        "()V",
        native_throwable_init,
    );
    registry.natives_mut().register(
        "java/lang/RuntimeException",
        "<init>",
        "(Ljava/lang/String;)V",
        native_throwable_init_string,
    );
    registry.natives_mut().register(
        "java/lang/RuntimeException",
        "<init>",
        "(Ljava/lang/String;Ljava/lang/Throwable;)V",
        native_throwable_init_string_cause,
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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(illegal_argument_ctx);
    registry.natives_mut().register(
        "java/lang/IllegalArgumentException",
        "<init>",
        "()V",
        native_throwable_init,
    );
    registry.natives_mut().register(
        "java/lang/IllegalArgumentException",
        "<init>",
        "(Ljava/lang/String;)V",
        native_throwable_init_string,
    );
    registry.natives_mut().register(
        "java/lang/IllegalArgumentException",
        "<init>",
        "(Ljava/lang/String;Ljava/lang/Throwable;)V",
        native_throwable_init_string_cause,
    );

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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(invocation_target_ctx);

    // Common RuntimeException subtypes — needed so materialize_java_exception_object
    // can allocate and hierarchy-check catches for NPE, CCE, AIOOB, etc.
    for (name, super_name) in [
        (
            "java/lang/NullPointerException",
            "java/lang/RuntimeException",
        ),
        ("java/lang/ClassCastException", "java/lang/RuntimeException"),
        (
            "java/lang/ArithmeticException",
            "java/lang/RuntimeException",
        ),
        (
            "java/lang/IndexOutOfBoundsException",
            "java/lang/RuntimeException",
        ),
        (
            "java/lang/ArrayIndexOutOfBoundsException",
            "java/lang/IndexOutOfBoundsException",
        ),
        (
            "java/lang/StringIndexOutOfBoundsException",
            "java/lang/IndexOutOfBoundsException",
        ),
        (
            "java/lang/UnsupportedOperationException",
            "java/lang/RuntimeException",
        ),
        (
            "java/lang/IllegalMonitorStateException",
            "java/lang/RuntimeException",
        ),
        (
            "java/lang/IllegalStateException",
            "java/lang/RuntimeException",
        ),
        (
            "java/lang/NumberFormatException",
            "java/lang/IllegalArgumentException",
        ),
        (
            "java/util/regex/PatternSyntaxException",
            "java/lang/IllegalArgumentException",
        ),
    ] {
        let ctx = ClassContext {
            class_name: name.to_string(),
            super_class: Some(super_name.to_string()),
            constant_pool: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            static_fields: Vec::new(),
            instance_field_count: 0,
            interfaces: Vec::new(),
            bootstrap_methods: Vec::new(),
            load_source: ClassLoadSource::Synthetic,
        };
        registry.register(ctx);
        registry
            .natives_mut()
            .register(name, "<init>", "()V", native_throwable_init);
        registry.natives_mut().register(
            name,
            "<init>",
            "(Ljava/lang/String;)V",
            native_throwable_init_string,
        );
        registry.natives_mut().register(
            name,
            "<init>",
            "(Ljava/lang/String;Ljava/lang/Throwable;)V",
            native_throwable_init_string_cause,
        );
    }

    // java/lang/Error and StackOverflowError
    for (name, super_name) in [
        ("java/lang/Error", "java/lang/Throwable"),
        ("java/lang/VirtualMachineError", "java/lang/Error"),
        (
            "java/lang/StackOverflowError",
            "java/lang/VirtualMachineError",
        ),
        (
            "java/lang/OutOfMemoryError",
            "java/lang/VirtualMachineError",
        ),
        ("java/lang/AssertionError", "java/lang/Error"),
        ("java/util/ServiceConfigurationError", "java/lang/Error"),
        // Linkage errors (JVMS 5.4/5.5): thrown when execution-time class
        // resolution or class initialisation fails. Registered here (with their
        // Throwable `<init>` natives) so bytecode can allocate them and, more
        // importantly, so `catch (LinkageError)` / `catch (Throwable)` in real
        // library code (e.g. commons-logging's Log4jApiLogFactory fallback)
        // match a synthesised NoClassDefFoundError via the super-class chain.
        ("java/lang/LinkageError", "java/lang/Error"),
        ("java/lang/NoClassDefFoundError", "java/lang/LinkageError"),
        (
            "java/lang/ExceptionInInitializerError",
            "java/lang/LinkageError",
        ),
        // IncompatibleClassChangeError family (JVMS 5.4.3): thrown when
        // execution-time method/field resolution fails against a loaded class.
        // Registered (with hierarchy) so `catch (LinkageError)` /
        // `catch (IncompatibleClassChangeError)` / `catch (Throwable)` in running
        // bytecode matches a synthesised NoSuchMethodError via the super chain.
        // NoSuchFieldError is registered for hierarchy completeness only — no
        // lenient field-resolution path throws it today.
        (
            "java/lang/IncompatibleClassChangeError",
            "java/lang/LinkageError",
        ),
        (
            "java/lang/NoSuchMethodError",
            "java/lang/IncompatibleClassChangeError",
        ),
        (
            "java/lang/NoSuchFieldError",
            "java/lang/IncompatibleClassChangeError",
        ),
    ] {
        let ctx = ClassContext {
            class_name: name.to_string(),
            super_class: Some(super_name.to_string()),
            constant_pool: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            static_fields: Vec::new(),
            instance_field_count: 0,
            interfaces: Vec::new(),
            bootstrap_methods: Vec::new(),
            load_source: ClassLoadSource::Synthetic,
        };
        registry.register(ctx);
        registry
            .natives_mut()
            .register(name, "<init>", "()V", native_throwable_init);
        registry.natives_mut().register(
            name,
            "<init>",
            "(Ljava/lang/String;)V",
            native_throwable_init_string,
        );
        registry.natives_mut().register(
            name,
            "<init>",
            "(Ljava/lang/String;Ljava/lang/Throwable;)V",
            native_throwable_init_string_cause,
        );
    }

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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(autocloseable_ctx);

    // java/sql/Driver - marker interface for ServiceLoader-based JDBC smoke tests.
    let sql_driver_ctx = ClassContext {
        class_name: "java/sql/Driver".to_string(),
        super_class: None,
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(sql_driver_ctx);

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
        load_source: ClassLoadSource::Synthetic,
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

    // java/lang/Integer — boxed int with value field + numeric constants.
    // Extends java/lang/Number (not Object) so `checkcast`/`instanceof Number`
    // succeeds on boxed ints, e.g. gson's Number TypeAdapter bridge method.
    let integer_ctx = ClassContext {
        class_name: "java/lang/Integer".to_string(),
        super_class: Some("java/lang/Number".to_string()),
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
        load_source: ClassLoadSource::Synthetic,
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
        // Extends java/lang/Number (not Object) so `checkcast`/`instanceof
        // Number` succeeds on boxed longs, matching real Java.
        super_class: Some("java/lang/Number".to_string()),
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
        load_source: ClassLoadSource::Synthetic,
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
    registry
        .natives_mut()
        .register("java/lang/Long", "intValue", "()I", native_long_intvalue);
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
        // Extends java/lang/Number (not Object) so `checkcast`/`instanceof
        // Number` succeeds on boxed doubles, matching real Java.
        super_class: Some("java/lang/Number".to_string()),
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
        load_source: ClassLoadSource::Synthetic,
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
    registry.natives_mut().register(
        "java/lang/Double",
        "doubleToRawLongBits",
        "(D)J",
        native_double_double_to_raw_long_bits,
    );
    registry.natives_mut().register(
        "java/lang/Double",
        "doubleToLongBits",
        "(D)J",
        native_double_double_to_long_bits,
    );
    registry.natives_mut().register(
        "java/lang/Double",
        "longBitsToDouble",
        "(J)D",
        native_double_long_bits_to_double,
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
        // Extends java/lang/Number (not Object) so `checkcast`/`instanceof
        // Number` succeeds on boxed floats, matching real Java.
        super_class: Some("java/lang/Number".to_string()),
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
        load_source: ClassLoadSource::Synthetic,
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
        "floatToRawIntBits",
        "(F)I",
        native_float_float_to_raw_int_bits,
    );
    registry.natives_mut().register(
        "java/lang/Float",
        "floatToIntBits",
        "(F)I",
        native_float_float_to_int_bits,
    );
    registry.natives_mut().register(
        "java/lang/Float",
        "intBitsToFloat",
        "(I)F",
        native_float_int_bits_to_float,
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
        load_source: ClassLoadSource::Synthetic,
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
        // Extends java/lang/Number (not Object) so `checkcast`/`instanceof
        // Number` succeeds on boxed bytes, matching real Java.
        super_class: Some("java/lang/Number".to_string()),
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
        load_source: ClassLoadSource::Synthetic,
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
        // Extends java/lang/Number (not Object) so `checkcast`/`instanceof
        // Number` succeeds on boxed shorts, matching real Java.
        super_class: Some("java/lang/Number".to_string()),
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(sb_ctx);

    // StringBuilder.<init>()V
    registry
        .natives_mut()
        .register("java/lang/StringBuilder", "<init>", "()V", native_sb_init);
    // StringBuilder.<init>(int)V
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "<init>",
        "(I)V",
        native_sb_init_with_capacity,
    );
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
    // StringBuilder.append(CharSequence, int, int)
    registry.natives_mut().register(
        "java/lang/StringBuilder",
        "append",
        "(Ljava/lang/CharSequence;II)Ljava/lang/StringBuilder;",
        native_sb_append_charsequence_range,
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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(character_ctx);

    // Character.isDigit(C)Z and (I)Z — (I) variant used in IntStream.filter(Character::isDigit)
    for desc in ["(C)Z", "(I)Z"] {
        registry.natives_mut().register(
            "java/lang/Character",
            "isDigit",
            desc,
            native_char_is_digit,
        );
        registry.natives_mut().register(
            "java/lang/Character",
            "isLetter",
            desc,
            native_char_is_letter,
        );
        registry.natives_mut().register(
            "java/lang/Character",
            "isWhitespace",
            desc,
            native_char_is_whitespace,
        );
        registry.natives_mut().register(
            "java/lang/Character",
            "isUpperCase",
            desc,
            native_char_is_uppercase,
        );
        registry.natives_mut().register(
            "java/lang/Character",
            "isLowerCase",
            desc,
            native_char_is_lowercase,
        );
        registry.natives_mut().register(
            "java/lang/Character",
            "isLetterOrDigit",
            desc,
            native_char_is_letter_or_digit,
        );
    }
    // Character.toUpperCase(C)C and (I)I
    registry.natives_mut().register(
        "java/lang/Character",
        "toUpperCase",
        "(C)C",
        native_char_to_uppercase,
    );
    registry.natives_mut().register(
        "java/lang/Character",
        "toUpperCase",
        "(I)I",
        native_char_to_uppercase,
    );
    // Character.toLowerCase(C)C and (I)I
    registry.natives_mut().register(
        "java/lang/Character",
        "toLowerCase",
        "(C)C",
        native_char_to_lowercase,
    );
    registry.natives_mut().register(
        "java/lang/Character",
        "toLowerCase",
        "(I)I",
        native_char_to_lowercase,
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

    registry
        .natives_mut()
        .register("java/lang/Character", "digit", "(CI)I", native_char_digit);

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
        load_source: ClassLoadSource::Synthetic,
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

    let service_loader_ctx = ClassContext {
        class_name: "java/util/ServiceLoader".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "service".to_string(),
                descriptor: "Ljava/lang/Class;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "loader".to_string(),
                descriptor: "Ljava/lang/ClassLoader;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "providerCount".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 3,
        interfaces: vec!["java/lang/Iterable".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(service_loader_ctx);
    registry.natives_mut().register_callback(
        "java/util/ServiceLoader",
        "load",
        "(Ljava/lang/Class;)Ljava/util/ServiceLoader;",
        native_service_loader_load,
    );
    registry.natives_mut().register_callback(
        "java/util/ServiceLoader",
        "load",
        "(Ljava/lang/Class;Ljava/lang/ClassLoader;)Ljava/util/ServiceLoader;",
        native_service_loader_load_with_loader,
    );
    registry.natives_mut().register(
        "java/util/ServiceLoader",
        "iterator",
        "()Ljava/util/Iterator;",
        native_service_loader_iterator,
    );
    registry.natives_mut().register_callback(
        "java/util/ServiceLoader",
        "stream",
        "()Ljava/util/stream/Stream;",
        native_service_loader_stream,
    );

    let service_iter_ctx = ClassContext {
        class_name: "duke/util/ServiceLoaderIterator".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "service".to_string(),
                descriptor: "Ljava/lang/Class;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "loader".to_string(),
                descriptor: "Ljava/lang/ClassLoader;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "index".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "providerCount".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 4,
        interfaces: vec!["java/util/Iterator".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(service_iter_ctx);
    registry.natives_mut().register(
        "duke/util/ServiceLoaderIterator",
        "<init>",
        "()V",
        native_service_loader_iter_init,
    );
    registry.natives_mut().register(
        "duke/util/ServiceLoaderIterator",
        "hasNext",
        "()Z",
        native_service_loader_iter_has_next,
    );
    registry.natives_mut().register_callback(
        "duke/util/ServiceLoaderIterator",
        "next",
        "()Ljava/lang/Object;",
        native_service_loader_iter_next,
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
        load_source: ClassLoadSource::Synthetic,
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
        "(I)V",
        native_arraylist_init_with_capacity,
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
        "lastIndexOf",
        "(Ljava/lang/Object;)I",
        native_arraylist_last_index_of,
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
        load_source: ClassLoadSource::Synthetic,
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

    // java/util/concurrent/CopyOnWriteArrayList — synthetic list that mirrors the
    // ArrayList storage layout (fields[0] = size, fields[1..] = elements), so the
    // shared ArrayListIterator native works on it unchanged. Single-threaded
    // execution collapses copy-on-write semantics to plain in-place mutation.
    let cowal_ctx = ClassContext {
        class_name: "java/util/concurrent/CopyOnWriteArrayList".to_string(),
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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(cowal_ctx);
    registry.natives_mut().register(
        "java/util/concurrent/CopyOnWriteArrayList",
        "<init>",
        "()V",
        native_cowal_init,
    );
    registry.natives_mut().register(
        "java/util/concurrent/CopyOnWriteArrayList",
        "add",
        "(Ljava/lang/Object;)Z",
        native_cowal_add,
    );
    registry.natives_mut().register(
        "java/util/concurrent/CopyOnWriteArrayList",
        "addIfAbsent",
        "(Ljava/lang/Object;)Z",
        native_cowal_add_if_absent,
    );
    registry.natives_mut().register(
        "java/util/concurrent/CopyOnWriteArrayList",
        "get",
        "(I)Ljava/lang/Object;",
        native_cowal_get,
    );
    registry.natives_mut().register(
        "java/util/concurrent/CopyOnWriteArrayList",
        "size",
        "()I",
        native_cowal_size,
    );
    registry.natives_mut().register(
        "java/util/concurrent/CopyOnWriteArrayList",
        "contains",
        "(Ljava/lang/Object;)Z",
        native_cowal_contains,
    );
    registry.natives_mut().register(
        "java/util/concurrent/CopyOnWriteArrayList",
        "isEmpty",
        "()Z",
        native_cowal_is_empty,
    );
    registry.natives_mut().register(
        "java/util/concurrent/CopyOnWriteArrayList",
        "iterator",
        "()Ljava/util/Iterator;",
        native_arraylist_iterator,
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
        load_source: ClassLoadSource::Synthetic,
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
        "equals",
        "([I[I)Z",
        native_arrays_equals_int,
    );
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
        load_source: ClassLoadSource::Synthetic,
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

    // java/util/Hashtable — minimal synchronized-map ancestor for Properties.
    // Duke is single-threaded, so this deliberately reuses the HashMap layout and natives.
    let hashtable_ctx = ClassContext {
        class_name: "java/util/Hashtable".to_string(),
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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(hashtable_ctx);
    for (method, descriptor, handler) in [
        ("<init>", "()V", native_hashmap_init as NativeHandler),
        (
            "put",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            native_hashmap_put as NativeHandler,
        ),
        (
            "get",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            native_hashmap_get as NativeHandler,
        ),
        (
            "containsKey",
            "(Ljava/lang/Object;)Z",
            native_hashmap_contains_key as NativeHandler,
        ),
        ("size", "()I", native_hashmap_size as NativeHandler),
        (
            "remove",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            native_hashmap_remove as NativeHandler,
        ),
        ("isEmpty", "()Z", native_hashmap_is_empty as NativeHandler),
        (
            "keySet",
            "()Ljava/util/Set;",
            native_hashmap_key_set as NativeHandler,
        ),
        (
            "values",
            "()Ljava/util/Collection;",
            native_hashmap_values as NativeHandler,
        ),
        (
            "entrySet",
            "()Ljava/util/Set;",
            native_hashmap_entry_set as NativeHandler,
        ),
        ("clear", "()V", native_hashmap_clear as NativeHandler),
    ] {
        registry
            .natives_mut()
            .register("java/util/Hashtable", method, descriptor, handler);
    }

    // java/util/Properties — String-keyed map with optional defaults chain.
    // Layout: fields[0] inherited Hashtable size, fields[1] defaults, fields[2..] key/value pairs.
    let properties_ctx = ClassContext {
        class_name: "java/util/Properties".to_string(),
        super_class: Some("java/util/Hashtable".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "defaults".to_string(),
            descriptor: "Ljava/util/Properties;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: vec![
            "java/util/Map".to_string(),
            "java/util/Collection".to_string(),
        ],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(properties_ctx);
    let properties_enum_ctx = ClassContext {
        class_name: "duke/util/PropertiesEnumeration".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "index".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "count".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/Enumeration".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(properties_enum_ctx);
    for (method, descriptor, handler) in [
        ("<init>", "()V", native_properties_init as NativeHandler),
        (
            "<init>",
            "(Ljava/util/Properties;)V",
            native_properties_init_defaults as NativeHandler,
        ),
        (
            "setProperty",
            "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;",
            native_properties_set_property as NativeHandler,
        ),
        (
            "getProperty",
            "(Ljava/lang/String;)Ljava/lang/String;",
            native_properties_get_property as NativeHandler,
        ),
        (
            "getProperty",
            "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
            native_properties_get_property_default as NativeHandler,
        ),
        (
            "load",
            "(Ljava/io/InputStream;)V",
            native_properties_load as NativeHandler,
        ),
        (
            "store",
            "(Ljava/io/OutputStream;Ljava/lang/String;)V",
            native_properties_store as NativeHandler,
        ),
        (
            "propertyNames",
            "()Ljava/util/Enumeration;",
            native_properties_property_names as NativeHandler,
        ),
        (
            "stringPropertyNames",
            "()Ljava/util/Set;",
            native_properties_string_property_names as NativeHandler,
        ),
        ("size", "()I", native_properties_size as NativeHandler),
        (
            "isEmpty",
            "()Z",
            native_properties_is_empty as NativeHandler,
        ),
        (
            "containsKey",
            "(Ljava/lang/Object;)Z",
            native_properties_contains_key as NativeHandler,
        ),
        ("clear", "()V", native_properties_clear as NativeHandler),
        (
            "remove",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            native_properties_remove as NativeHandler,
        ),
        (
            "keySet",
            "()Ljava/util/Set;",
            native_properties_key_set as NativeHandler,
        ),
        (
            "values",
            "()Ljava/util/Collection;",
            native_properties_values as NativeHandler,
        ),
        (
            "entrySet",
            "()Ljava/util/Set;",
            native_properties_entry_set as NativeHandler,
        ),
        (
            "toString",
            "()Ljava/lang/String;",
            native_properties_to_string as NativeHandler,
        ),
    ] {
        registry
            .natives_mut()
            .register("java/util/Properties", method, descriptor, handler);
    }
    for (method, descriptor, handler) in [
        (
            "hasMoreElements",
            "()Z",
            native_properties_enum_has_more_elements as NativeHandler,
        ),
        (
            "nextElement",
            "()Ljava/lang/Object;",
            native_properties_enum_next_element as NativeHandler,
        ),
    ] {
        registry.natives_mut().register(
            "duke/util/PropertiesEnumeration",
            method,
            descriptor,
            handler,
        );
    }

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
        load_source: ClassLoadSource::Synthetic,
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
        "<init>",
        "(Ljava/util/Collection;)V",
        native_linked_list_init_collection,
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
    // peek() is an alias for peekFirst() in Queue context
    registry.natives_mut().register(
        "java/util/LinkedList",
        "peek",
        "()Ljava/lang/Object;",
        native_linked_list_peek_first,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        "shuffle",
        "(Ljava/util/List;Ljava/util/Random;)V",
        native_collections_shuffle_random,
    );
    registry.natives_mut().register(
        "java/util/Collections",
        "fill",
        "(Ljava/util/List;Ljava/lang/Object;)V",
        native_collections_fill,
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

    // java/util/UnmodifiableList — wrapper that throws UnsupportedOperationException on mutation
    let unmod_list_ctx = ClassContext {
        class_name: "java/util/UnmodifiableList".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec![
            "java/util/List".to_string(),
            "java/util/Collection".to_string(),
        ],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(unmod_list_ctx);
    // Read operations — reuse ArrayList handlers (same field layout)
    registry.natives_mut().register(
        "java/util/UnmodifiableList",
        "size",
        "()I",
        native_arraylist_size,
    );
    registry.natives_mut().register(
        "java/util/UnmodifiableList",
        "get",
        "(I)Ljava/lang/Object;",
        native_arraylist_get,
    );
    registry.natives_mut().register(
        "java/util/UnmodifiableList",
        "contains",
        "(Ljava/lang/Object;)Z",
        native_arraylist_contains,
    );
    registry.natives_mut().register(
        "java/util/UnmodifiableList",
        "isEmpty",
        "()Z",
        native_arraylist_is_empty,
    );
    registry.natives_mut().register(
        "java/util/UnmodifiableList",
        "iterator",
        "()Ljava/util/Iterator;",
        native_arraylist_iterator,
    );
    // Mutation operations — throw UnsupportedOperationException
    for (method, desc) in [
        ("add", "(Ljava/lang/Object;)Z"),
        ("add", "(ILjava/lang/Object;)V"),
        ("remove", "(I)Ljava/lang/Object;"),
        ("remove", "(Ljava/lang/Object;)Z"),
        ("set", "(ILjava/lang/Object;)Ljava/lang/Object;"),
        ("clear", "()V"),
    ] {
        registry.natives_mut().register(
            "java/util/UnmodifiableList",
            method,
            desc,
            native_unmodifiable_list_mutation,
        );
    }

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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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

    // Collections.unmodifiableMap(map) — wraps in UnmodifiableMap
    registry.natives_mut().register(
        "java/util/Collections",
        "unmodifiableMap",
        "(Ljava/util/Map;)Ljava/util/Map;",
        native_collections_unmodifiable_map,
    );
    // java/util/UnmodifiableMap — same layout as HashMap; reads delegate, writes throw
    let unmod_map_ctx = ClassContext {
        class_name: "java/util/UnmodifiableMap".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/util/Map".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(unmod_map_ctx);
    registry.natives_mut().register(
        "java/util/UnmodifiableMap",
        "get",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_hashmap_get,
    );
    registry.natives_mut().register(
        "java/util/UnmodifiableMap",
        "containsKey",
        "(Ljava/lang/Object;)Z",
        native_hashmap_contains_key,
    );
    registry.natives_mut().register(
        "java/util/UnmodifiableMap",
        "size",
        "()I",
        native_hashmap_size,
    );
    registry.natives_mut().register(
        "java/util/UnmodifiableMap",
        "entrySet",
        "()Ljava/util/Set;",
        native_hashmap_entry_set,
    );
    registry.natives_mut().register(
        "java/util/UnmodifiableMap",
        "keySet",
        "()Ljava/util/Set;",
        native_hashmap_key_set,
    );
    registry.natives_mut().register(
        "java/util/UnmodifiableMap",
        "values",
        "()Ljava/util/Collection;",
        native_hashmap_values,
    );
    // Mutation ops throw UnsupportedOperationException
    for (method, desc) in [
        (
            "put",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        ),
        ("remove", "(Ljava/lang/Object;)Ljava/lang/Object;"),
        ("clear", "()V"),
    ] {
        registry.natives_mut().register(
            "java/util/UnmodifiableMap",
            method,
            desc,
            native_unmodifiable_list_mutation,
        );
    }
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
    registry.natives_mut().register_callback(
        "java/util/stream/Collectors",
        "partitioningBy",
        "(Ljava/util/function/Predicate;Ljava/util/stream/Collector;)Ljava/util/stream/Collector;",
        native_collectors_partitioning_by_downstream,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
    registry.natives_mut().register(
        "java/util/TreeMap",
        "headMap",
        "(Ljava/lang/Object;)Ljava/util/SortedMap;",
        native_treemap_head_map,
    );
    registry.natives_mut().register(
        "java/util/TreeMap",
        "tailMap",
        "(Ljava/lang/Object;)Ljava/util/SortedMap;",
        native_treemap_tail_map,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
    // UnmodifiableList.stream() — same layout as ArrayList
    registry.natives_mut().register(
        "java/util/UnmodifiableList",
        "stream",
        "()Ljava/util/stream/Stream;",
        native_arraylist_stream,
    );
    // UnmodifiableList.forEach() — same layout as ArrayList
    registry.natives_mut().register_callback(
        "java/util/UnmodifiableList",
        "forEach",
        "(Ljava/util/function/Consumer;)V",
        native_arraylist_for_each,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(to_list_ctx);

    // duke/util/ToUnmodifiableListCollector — sentinel for Collectors.toUnmodifiableList()
    let to_unmod_list_ctx = ClassContext {
        class_name: "duke/util/ToUnmodifiableListCollector".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/util/stream/Collector".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(to_unmod_list_ctx);

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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
    // IntStream.iterate(seed, UnaryOperator) — generates up to 4096 elements (limit truncates)
    registry.natives_mut().register_callback(
        "java/util/stream/IntStream",
        "iterate",
        "(ILjava/util/function/IntUnaryOperator;)Ljava/util/stream/IntStream;",
        native_int_stream_iterate,
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
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "peek",
        "(Ljava/util/function/IntConsumer;)Ljava/util/stream/IntStream;",
        native_int_stream_peek,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
    registry.natives_mut().register(
        "duke/util/OptionalInt",
        "orElse",
        "(I)I",
        native_optional_int_or_else,
    );
    // OptionalInt.of(int) and OptionalInt.empty() static factories
    registry.natives_mut().register(
        "java/util/OptionalInt",
        "of",
        "(I)Ljava/util/OptionalInt;",
        native_optional_int_of,
    );
    registry.natives_mut().register(
        "java/util/OptionalInt",
        "empty",
        "()Ljava/util/OptionalInt;",
        native_optional_int_empty,
    );
    // Also on duke/util/OptionalInt
    registry.natives_mut().register(
        "duke/util/OptionalInt",
        "of",
        "(I)Ljava/util/OptionalInt;",
        native_optional_int_of,
    );
    registry.natives_mut().register(
        "duke/util/OptionalInt",
        "empty",
        "()Ljava/util/OptionalInt;",
        native_optional_int_empty,
    );
    // mirror getAsInt/isPresent/orElse on java/util/OptionalInt too
    registry.natives_mut().register(
        "java/util/OptionalInt",
        "getAsInt",
        "()I",
        native_optional_int_get_as_int,
    );
    registry.natives_mut().register(
        "java/util/OptionalInt",
        "isPresent",
        "()Z",
        native_optional_int_is_present,
    );
    registry.natives_mut().register(
        "java/util/OptionalInt",
        "orElse",
        "(I)I",
        native_optional_int_or_else,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(j_opt_dbl_ctx);
    registry.natives_mut().register(
        "duke/util/OptionalDouble",
        "getAsDouble",
        "()D",
        native_optional_double_get_as_double,
    );
    registry.natives_mut().register(
        "duke/util/OptionalDouble",
        "orElse",
        "(D)D",
        native_optional_double_or_else,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        "toString",
        "(Ljava/lang/Object;Ljava/lang/String;)Ljava/lang/String;",
        native_objects_tostring_default,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
    let pattern_flag_fields = [
        ("UNIX_LINES", 1),
        ("CASE_INSENSITIVE", 2),
        ("COMMENTS", 4),
        ("MULTILINE", 8),
        ("LITERAL", 16),
        ("DOTALL", 32),
        ("UNICODE_CASE", 64),
        ("CANON_EQ", 128),
        ("UNICODE_CHARACTER_CLASS", 256),
    ];
    let mut pattern_fields: Vec<FieldEntry> = pattern_flag_fields
        .iter()
        .map(|(name, _)| synthetic_field(name, "I", true))
        .collect();
    pattern_fields.push(synthetic_field("flags", "I", false));
    let pattern_static_fields: Vec<Slot> = pattern_flag_fields
        .iter()
        .map(|(_, value)| Slot::Int(*value))
        .collect();
    let pattern_ctx = ClassContext {
        class_name: "java/util/regex/Pattern".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: pattern_fields,
        static_fields: pattern_static_fields,
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
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
        "compile",
        "(Ljava/lang/String;I)Ljava/util/regex/Pattern;",
        native_pattern_compile_flags,
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
    registry.natives_mut().register(
        "java/util/regex/Pattern",
        "split",
        "(Ljava/lang/CharSequence;)[Ljava/lang/String;",
        native_pattern_split,
    );
    registry.natives_mut().register(
        "java/util/regex/Pattern",
        "split",
        "(Ljava/lang/CharSequence;I)[Ljava/lang/String;",
        native_pattern_split_limit,
    );

    // java/util/regex/Matcher — stateful matcher
    // fields[0]=Pattern, [1]=input, [2]=pos, [3]=match_start, [4]=match_end,
    // [5]=append_pos
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
            FieldEntry {
                name: "appendPos".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 6,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
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
        "group",
        "(I)Ljava/lang/String;",
        native_matcher_group_n,
    );
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "group",
        "(Ljava/lang/String;)Ljava/lang/String;",
        native_matcher_group_name,
    );
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "groupCount",
        "()I",
        native_matcher_group_count,
    );
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "start",
        "()I",
        native_matcher_start,
    );
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "start",
        "(Ljava/lang/String;)I",
        native_matcher_start_name,
    );
    registry
        .natives_mut()
        .register("java/util/regex/Matcher", "end", "()I", native_matcher_end);
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "end",
        "(Ljava/lang/String;)I",
        native_matcher_end_name,
    );
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "reset",
        "()Ljava/util/regex/Matcher;",
        native_matcher_reset,
    );
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "reset",
        "(Ljava/lang/CharSequence;)Ljava/util/regex/Matcher;",
        native_matcher_reset_input,
    );
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "appendReplacement",
        "(Ljava/lang/StringBuilder;Ljava/lang/String;)Ljava/util/regex/Matcher;",
        native_matcher_append_replacement_sb,
    );
    registry.natives_mut().register(
        "java/util/regex/Matcher",
        "appendTail",
        "(Ljava/lang/StringBuilder;)Ljava/lang/StringBuilder;",
        native_matcher_append_tail_sb,
    );
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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

    // java/util/UUID — immutable 128-bit value type
    // fields[0] = most significant bits, fields[1] = least significant bits
    let uuid_ctx = ClassContext {
        class_name: "java/util/UUID".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "mostSigBits".to_string(),
                descriptor: "J".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "leastSigBits".to_string(),
                descriptor: "J".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec![
            "java/io/Serializable".to_string(),
            "java/lang/Comparable".to_string(),
        ],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(uuid_ctx);
    for (method, descriptor, handler) in [
        ("<init>", "(JJ)V", native_uuid_init as NativeHandler),
        (
            "randomUUID",
            "()Ljava/util/UUID;",
            native_uuid_random_uuid as NativeHandler,
        ),
        (
            "nameUUIDFromBytes",
            "([B)Ljava/util/UUID;",
            native_uuid_name_uuid_from_bytes as NativeHandler,
        ),
        (
            "fromString",
            "(Ljava/lang/String;)Ljava/util/UUID;",
            native_uuid_from_string as NativeHandler,
        ),
        (
            "getMostSignificantBits",
            "()J",
            native_uuid_get_most_significant_bits as NativeHandler,
        ),
        (
            "getLeastSignificantBits",
            "()J",
            native_uuid_get_least_significant_bits as NativeHandler,
        ),
        ("version", "()I", native_uuid_version as NativeHandler),
        ("variant", "()I", native_uuid_variant as NativeHandler),
        (
            "toString",
            "()Ljava/lang/String;",
            native_uuid_to_string as NativeHandler,
        ),
        (
            "equals",
            "(Ljava/lang/Object;)Z",
            native_uuid_equals as NativeHandler,
        ),
        ("hashCode", "()I", native_uuid_hash_code as NativeHandler),
        (
            "compareTo",
            "(Ljava/util/UUID;)I",
            native_uuid_compare_to as NativeHandler,
        ),
        (
            "compareTo",
            "(Ljava/lang/Object;)I",
            native_uuid_compare_to as NativeHandler,
        ),
        (
            "timestamp",
            "()J",
            native_uuid_unsupported_version1_accessor as NativeHandler,
        ),
        (
            "clockSequence",
            "()I",
            native_uuid_unsupported_version1_accessor as NativeHandler,
        ),
        (
            "node",
            "()J",
            native_uuid_unsupported_version1_accessor as NativeHandler,
        ),
    ] {
        registry
            .natives_mut()
            .register("java/util/UUID", method, descriptor, handler);
    }

    for name in [
        "java/security/PrivilegedAction",
        "java/security/PrivilegedExceptionAction",
    ] {
        registry.register(ClassContext {
            class_name: name.to_string(),
            super_class: Some("java/lang/Object".to_string()),
            constant_pool: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            static_fields: Vec::new(),
            instance_field_count: 0,
            interfaces: Vec::new(),
            bootstrap_methods: Vec::new(),
            load_source: ClassLoadSource::Synthetic,
        });
    }

    let access_controller_ctx = ClassContext {
        class_name: "java/security/AccessController".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(access_controller_ctx);
    registry.natives_mut().register_callback(
        "java/security/AccessController",
        "doPrivileged",
        "(Ljava/security/PrivilegedAction;)Ljava/lang/Object;",
        native_access_controller_do_privileged_action,
    );
    registry.natives_mut().register_callback(
        "java/security/AccessController",
        "doPrivileged",
        "(Ljava/security/PrivilegedExceptionAction;)Ljava/lang/Object;",
        native_access_controller_do_privileged_exception_action,
    );

    let privileged_action_exception_ctx = ClassContext {
        class_name: "java/security/PrivilegedActionException".to_string(),
        super_class: Some("java/lang/Exception".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(privileged_action_exception_ctx);
    registry.natives_mut().register(
        "java/security/PrivilegedActionException",
        "<init>",
        "(Ljava/lang/Exception;)V",
        native_throwable_init_cause,
    );
    registry.natives_mut().register(
        "java/security/PrivilegedActionException",
        "getException",
        "()Ljava/lang/Exception;",
        native_throwable_get_cause,
    );

    let no_such_algorithm_ctx = ClassContext {
        class_name: "java/security/NoSuchAlgorithmException".to_string(),
        super_class: Some("java/lang/Exception".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(no_such_algorithm_ctx);

    let no_such_provider_ctx = ClassContext {
        class_name: "java/security/NoSuchProviderException".to_string(),
        super_class: Some("java/lang/Exception".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(no_such_provider_ctx);

    let security_ctx = ClassContext {
        class_name: "java/security/Security".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(security_ctx);
    registry.natives_mut().register(
        "java/security/Security",
        "getProvider",
        "(Ljava/lang/String;)Ljava/security/Provider;",
        native_security_get_provider,
    );
    registry.natives_mut().register(
        "java/security/Security",
        "getProviders",
        "()[Ljava/security/Provider;",
        native_security_get_providers,
    );
    registry.natives_mut().register(
        "java/security/Security",
        "getAlgorithms",
        "(Ljava/lang/String;)Ljava/util/Set;",
        native_security_get_algorithms,
    );

    let provider_ctx = ClassContext {
        class_name: "java/security/Provider".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(provider_ctx);
    registry.natives_mut().register(
        "java/security/Provider",
        "getName",
        "()Ljava/lang/String;",
        native_provider_get_name,
    );
    registry.natives_mut().register(
        "java/security/Provider",
        "getService",
        "(Ljava/lang/String;Ljava/lang/String;)Ljava/security/Provider$Service;",
        native_provider_get_service,
    );

    let provider_service_ctx = ClassContext {
        class_name: "java/security/Provider$Service".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "provider".to_string(),
                descriptor: "Ljava/security/Provider;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "type".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "algorithm".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 3,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(provider_service_ctx);
    registry.natives_mut().register(
        "java/security/Provider$Service",
        "getAlgorithm",
        "()Ljava/lang/String;",
        native_provider_service_get_algorithm,
    );
    registry.natives_mut().register(
        "java/security/Provider$Service",
        "getType",
        "()Ljava/lang/String;",
        native_provider_service_get_type,
    );
    registry.natives_mut().register(
        "java/security/Provider$Service",
        "getProvider",
        "()Ljava/security/Provider;",
        native_provider_service_get_provider,
    );

    let secure_random_ctx = ClassContext {
        class_name: "java/security/SecureRandom".to_string(),
        super_class: Some("java/util/Random".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(secure_random_ctx);
    registry.natives_mut().register(
        "java/security/SecureRandom",
        "nextBytes",
        "([B)V",
        native_secure_random_next_bytes,
    );
    registry.natives_mut().register(
        "java/security/SecureRandom",
        "generateSeed",
        "(I)[B",
        native_secure_random_generate_seed,
    );

    let message_digest_ctx = ClassContext {
        class_name: "java/security/MessageDigest".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "algorithm".to_string(),
                descriptor: "Ljava/lang/String;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "buffer".to_string(),
                descriptor: "[B".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(message_digest_ctx);
    registry.natives_mut().register(
        "java/security/MessageDigest",
        "getInstance",
        "(Ljava/lang/String;)Ljava/security/MessageDigest;",
        native_message_digest_get_instance,
    );
    registry.natives_mut().register(
        "java/security/MessageDigest",
        "getInstance",
        "(Ljava/lang/String;Ljava/lang/String;)Ljava/security/MessageDigest;",
        native_message_digest_get_instance_provider_name,
    );
    registry.natives_mut().register(
        "java/security/MessageDigest",
        "getInstance",
        "(Ljava/lang/String;Ljava/security/Provider;)Ljava/security/MessageDigest;",
        native_message_digest_get_instance_provider,
    );
    registry.natives_mut().register(
        "java/security/MessageDigest",
        "getAlgorithm",
        "()Ljava/lang/String;",
        native_message_digest_get_algorithm,
    );
    registry.natives_mut().register(
        "java/security/MessageDigest",
        "getProvider",
        "()Ljava/security/Provider;",
        native_message_digest_get_provider,
    );
    registry.natives_mut().register(
        "java/security/MessageDigest",
        "update",
        "(B)V",
        native_message_digest_update_byte,
    );
    registry.natives_mut().register(
        "java/security/MessageDigest",
        "update",
        "([B)V",
        native_message_digest_update_bytes,
    );
    registry.natives_mut().register(
        "java/security/MessageDigest",
        "update",
        "([BII)V",
        native_message_digest_update_bytes_range,
    );
    registry.natives_mut().register(
        "java/security/MessageDigest",
        "digest",
        "([B)[B",
        native_message_digest_digest_bytes,
    );
    registry.natives_mut().register(
        "java/security/MessageDigest",
        "digest",
        "()[B",
        native_message_digest_digest,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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

    // Consumer.andThen combinator
    registry.natives_mut().register(
        "java/util/function/Consumer",
        "andThen",
        "(Ljava/util/function/Consumer;)Ljava/util/function/Consumer;",
        native_consumer_and_then,
    );
    let and_then_consumer_ctx = ClassContext {
        class_name: "duke/util/AndThenConsumer".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "first".to_string(),
                descriptor: "Ljava/util/function/Consumer;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "second".to_string(),
                descriptor: "Ljava/util/function/Consumer;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/function/Consumer".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(and_then_consumer_ctx);
    registry.natives_mut().register_callback(
        "duke/util/AndThenConsumer",
        "accept",
        "(Ljava/lang/Object;)V",
        native_and_then_consumer_accept,
    );
    registry.natives_mut().register(
        "duke/util/AndThenConsumer",
        "andThen",
        "(Ljava/util/function/Consumer;)Ljava/util/function/Consumer;",
        native_consumer_and_then,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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

    // BiFunction.andThen combinator
    let bifunction_ctx = ClassContext {
        class_name: "java/util/function/BiFunction".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(bifunction_ctx);
    registry.natives_mut().register(
        "java/util/function/BiFunction",
        "andThen",
        "(Ljava/util/function/Function;)Ljava/util/function/BiFunction;",
        native_bifunction_and_then,
    );
    let bifunction_and_then_ctx = ClassContext {
        class_name: "duke/util/BiFunctionAndThen".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "bifunction".to_string(),
                descriptor: "Ljava/util/function/BiFunction;".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "after".to_string(),
                descriptor: "Ljava/util/function/Function;".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 2,
        interfaces: vec!["java/util/function/BiFunction".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(bifunction_and_then_ctx);
    registry.natives_mut().register_callback(
        "duke/util/BiFunctionAndThen",
        "apply",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        native_bifunction_and_then_apply,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
    registry.natives_mut().register(
        "duke/util/OptionalLong",
        "orElse",
        "(J)J",
        native_optional_long_or_else,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
    registry.register(empty_synthetic_context(
        "java/time/DateTimeException",
        "java/lang/RuntimeException",
    ));
    registry.register(empty_synthetic_context(
        "java/time/format/DateTimeParseException",
        "java/time/DateTimeException",
    ));

    let iso_instant_formatter_ref =
        heap.allocate("java/time/format/DateTimeFormatter".to_string(), 0);
    if let Ok(formatter) = heap.get_mut(iso_instant_formatter_ref) {
        formatter.string_value = Some("ISO_INSTANT".to_string());
    }
    let iso_local_date_formatter_ref =
        heap.allocate("java/time/format/DateTimeFormatter".to_string(), 0);
    if let Ok(formatter) = heap.get_mut(iso_local_date_formatter_ref) {
        formatter.string_value = Some("ISO_LOCAL_DATE".to_string());
    }
    let iso_local_date_time_formatter_ref =
        heap.allocate("java/time/format/DateTimeFormatter".to_string(), 0);
    if let Ok(formatter) = heap.get_mut(iso_local_date_time_formatter_ref) {
        formatter.string_value = Some("ISO_LOCAL_DATE_TIME".to_string());
    }
    let date_time_formatter_ctx = ClassContext {
        class_name: "java/time/format/DateTimeFormatter".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            synthetic_field("ISO_INSTANT", "Ljava/time/format/DateTimeFormatter;", true),
            synthetic_field(
                "ISO_LOCAL_DATE",
                "Ljava/time/format/DateTimeFormatter;",
                true,
            ),
            synthetic_field(
                "ISO_LOCAL_DATE_TIME",
                "Ljava/time/format/DateTimeFormatter;",
                true,
            ),
        ],
        static_fields: vec![
            Slot::Reference(Some(iso_instant_formatter_ref)),
            Slot::Reference(Some(iso_local_date_formatter_ref)),
            Slot::Reference(Some(iso_local_date_time_formatter_ref)),
        ],
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(date_time_formatter_ctx);

    // Minimal-for-boot DateTimeFormatter natives. logback's `CachingDateFormatter`
    // builds a pattern formatter (`ofPattern`), binds it to a zone/locale
    // (`withZone`/`withLocale` — no-ops under duke's UTC-only, fixed en-US model),
    // then formats an `Instant` via `format(TemporalAccessor)`. See the pattern
    // engine and simplifications documented in native/java_time.rs.
    registry.natives_mut().register(
        "java/time/format/DateTimeFormatter",
        "ofPattern",
        "(Ljava/lang/String;)Ljava/time/format/DateTimeFormatter;",
        native_datetimeformatter_of_pattern,
    );
    registry.natives_mut().register(
        "java/time/format/DateTimeFormatter",
        "withZone",
        "(Ljava/time/ZoneId;)Ljava/time/format/DateTimeFormatter;",
        native_datetimeformatter_with_zone,
    );
    registry.natives_mut().register(
        "java/time/format/DateTimeFormatter",
        "withLocale",
        "(Ljava/util/Locale;)Ljava/time/format/DateTimeFormatter;",
        native_datetimeformatter_with_locale,
    );
    registry.natives_mut().register(
        "java/time/format/DateTimeFormatter",
        "format",
        "(Ljava/time/temporal/TemporalAccessor;)Ljava/lang/String;",
        native_datetimeformatter_format,
    );

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
        load_source: ClassLoadSource::Synthetic,
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
    registry.natives_mut().register(
        "java/time/LocalDate",
        "minusMonths",
        "(J)Ljava/time/LocalDate;",
        native_localdate_minus_months,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "withYear",
        "(I)Ljava/time/LocalDate;",
        native_localdate_with_year,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "parse",
        "(Ljava/lang/CharSequence;)Ljava/time/LocalDate;",
        native_localdate_parse,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "parse",
        "(Ljava/lang/CharSequence;Ljava/time/format/DateTimeFormatter;)Ljava/time/LocalDate;",
        native_localdate_parse_with_formatter,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "compareTo",
        "(Ljava/time/chrono/ChronoLocalDate;)I",
        native_localdate_compare_to,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "compareTo",
        "(Ljava/lang/Object;)I",
        native_localdate_compare_to_object,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "equals",
        "(Ljava/lang/Object;)Z",
        native_localdate_equals,
    );
    registry.natives_mut().register(
        "java/time/LocalDate",
        "hashCode",
        "()I",
        native_localdate_hash_code,
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
        load_source: ClassLoadSource::Synthetic,
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
    registry.natives_mut().register(
        "java/time/Duration",
        "ofMillis",
        "(J)Ljava/time/Duration;",
        native_duration_of_millis,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "ofNanos",
        "(J)Ljava/time/Duration;",
        native_duration_of_nanos,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "between",
        "(Ljava/time/temporal/Temporal;Ljava/time/temporal/Temporal;)Ljava/time/Duration;",
        native_duration_between,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "toMillis",
        "()J",
        native_duration_to_millis,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "toNanos",
        "()J",
        native_duration_to_nanos,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "negated",
        "()Ljava/time/Duration;",
        native_duration_negated,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "compareTo",
        "(Ljava/time/Duration;)I",
        native_duration_compare_to,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "compareTo",
        "(Ljava/lang/Object;)I",
        native_duration_compare_to_object,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "equals",
        "(Ljava/lang/Object;)Z",
        native_duration_equals,
    );
    registry.natives_mut().register(
        "java/time/Duration",
        "hashCode",
        "()I",
        native_duration_hash_code,
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
        load_source: ClassLoadSource::Synthetic,
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
        load_source: ClassLoadSource::Synthetic,
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
    registry.natives_mut().register(
        "java/time/Instant",
        "ofEpochSecond",
        "(JJ)Ljava/time/Instant;",
        native_instant_of_epoch_second_nanos,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "getNano",
        "()I",
        native_instant_get_nano,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "now",
        "()Ljava/time/Instant;",
        native_instant_now,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "plusSeconds",
        "(J)Ljava/time/Instant;",
        native_instant_plus_seconds,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "plusNanos",
        "(J)Ljava/time/Instant;",
        native_instant_plus_nanos,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "minusMillis",
        "(J)Ljava/time/Instant;",
        native_instant_minus_millis,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "parse",
        "(Ljava/lang/CharSequence;)Ljava/time/Instant;",
        native_instant_parse,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "toString",
        "()Ljava/lang/String;",
        native_instant_to_string,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "compareTo",
        "(Ljava/time/Instant;)I",
        native_instant_compare_to,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "compareTo",
        "(Ljava/lang/Object;)I",
        native_instant_compare_to_object,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "equals",
        "(Ljava/lang/Object;)Z",
        native_instant_equals,
    );
    registry.natives_mut().register(
        "java/time/Instant",
        "hashCode",
        "()I",
        native_instant_hash_code,
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
        load_source: ClassLoadSource::Synthetic,
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
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "plusHours",
        "(J)Ljava/time/LocalDateTime;",
        native_localdatetime_plus_hours,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "parse",
        "(Ljava/lang/CharSequence;)Ljava/time/LocalDateTime;",
        native_localdatetime_parse,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "parse",
        "(Ljava/lang/CharSequence;Ljava/time/format/DateTimeFormatter;)Ljava/time/LocalDateTime;",
        native_localdatetime_parse_with_formatter,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "compareTo",
        "(Ljava/time/chrono/ChronoLocalDateTime;)I",
        native_localdatetime_compare_to,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "compareTo",
        "(Ljava/lang/Object;)I",
        native_localdatetime_compare_to_object,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "equals",
        "(Ljava/lang/Object;)Z",
        native_localdatetime_equals,
    );
    registry.natives_mut().register(
        "java/time/LocalDateTime",
        "hashCode",
        "()I",
        native_localdatetime_hash_code,
    );

    // ---- java.time.ZoneId (minimal-for-boot) ----
    // Spring Boot / logback's timestamp formatting reaches `ZoneId` during startup.
    // Duke does not model a real tzdb, so `ZoneId` is a thin synthetic holder for a
    // zone-id string. `systemDefault()` reports UTC: the interpreter's clocks are all
    // epoch/UTC based (see `Instant`/`LocalDate*` natives above), so a UTC default
    // zone keeps timestamps self-consistent without pulling in `ZoneRules`/tzdb.
    let zone_id_ctx = ClassContext {
        class_name: "java/time/ZoneId".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        // one field: the zone-id string (stored via string_value on the instance)
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(zone_id_ctx);
    registry.natives_mut().register(
        "java/time/ZoneId",
        "systemDefault",
        "()Ljava/time/ZoneId;",
        native_zoneid_system_default,
    );
    registry.natives_mut().register(
        "java/time/ZoneId",
        "of",
        "(Ljava/lang/String;)Ljava/time/ZoneId;",
        native_zoneid_of,
    );
    registry.natives_mut().register(
        "java/time/ZoneId",
        "getId",
        "()Ljava/lang/String;",
        native_zoneid_get_id,
    );
    registry.natives_mut().register(
        "java/time/ZoneId",
        "toString",
        "()Ljava/lang/String;",
        native_zoneid_get_id,
    );

    // ---- java.util.Locale (minimal-for-boot) ----
    // logback's `CachingDateFormatter` calls `Locale.getDefault()` during startup.
    // Duke does not model CLDR / ResourceBundle, so `Locale` is a thin synthetic holder
    // for a language tag + country code (see the natives in native/java_util.rs).
    // `getDefault()` reports a fixed en-US locale, keeping boot deterministic without a
    // locale/CLDR data build-out.
    let locale_ctx = ClassContext {
        class_name: "java/util/Locale".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        // fields[0] = language String, fields[1] = country String
        instance_field_count: 2,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(locale_ctx);
    registry.natives_mut().register(
        "java/util/Locale",
        "getDefault",
        "()Ljava/util/Locale;",
        native_locale_get_default,
    );
    registry.natives_mut().register(
        "java/util/Locale",
        "getDefault",
        "(Ljava/util/Locale$Category;)Ljava/util/Locale;",
        native_locale_get_default_category,
    );
    registry.natives_mut().register(
        "java/util/Locale",
        "getLanguage",
        "()Ljava/lang/String;",
        native_locale_get_language,
    );
    registry.natives_mut().register(
        "java/util/Locale",
        "getCountry",
        "()Ljava/lang/String;",
        native_locale_get_country,
    );
    registry.natives_mut().register(
        "java/util/Locale",
        "toString",
        "()Ljava/lang/String;",
        native_locale_to_string,
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

    // Phase 66 additions
    registry.natives_mut().register_callback(
        "java/util/HashMap",
        "replaceAll",
        "(Ljava/util/function/BiFunction;)V",
        native_hashmap_replace_all,
    );
    registry.natives_mut().register(
        "duke/util/ArrayListIterator",
        "remove",
        "()V",
        native_arraylist_iter_remove,
    );

    // Phase 88 additions

    // Phase 100: Collectors.summarizingInt → IntSummaryStatistics
    registry.natives_mut().register(
        "java/util/stream/Collectors",
        "summarizingInt",
        "(Ljava/util/function/ToIntFunction;)Ljava/util/stream/Collector;",
        native_collectors_summarizing_int,
    );
    let summarizing_ctx = ClassContext {
        class_name: "duke/util/SummarizingIntCollector".to_string(),
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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(summarizing_ctx);
    // IntSummaryStatistics — fields: [0]=count(J) [1]=sum(J) [2]=min(I) [3]=max(I)
    let iss_ctx = ClassContext {
        class_name: "java/util/IntSummaryStatistics".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![
            FieldEntry {
                name: "count".to_string(),
                descriptor: "J".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "sum".to_string(),
                descriptor: "J".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "min".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
            FieldEntry {
                name: "max".to_string(),
                descriptor: "I".to_string(),
                is_static: false,
            },
        ],
        static_fields: Vec::new(),
        instance_field_count: 4,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(iss_ctx);
    registry.natives_mut().register(
        "java/util/IntSummaryStatistics",
        "getCount",
        "()J",
        native_int_summary_stats_get_count,
    );
    registry.natives_mut().register(
        "java/util/IntSummaryStatistics",
        "getSum",
        "()J",
        native_int_summary_stats_get_sum,
    );
    registry.natives_mut().register(
        "java/util/IntSummaryStatistics",
        "getMin",
        "()I",
        native_int_summary_stats_get_min,
    );
    registry.natives_mut().register(
        "java/util/IntSummaryStatistics",
        "getMax",
        "()I",
        native_int_summary_stats_get_max,
    );
    registry.natives_mut().register(
        "java/util/IntSummaryStatistics",
        "getAverage",
        "()D",
        native_int_summary_stats_get_average,
    );

    // java/util/BitSet — bitmask stored as fields[0] = Long(bits)
    let bitset_ctx = ClassContext {
        class_name: "java/util/BitSet".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "bits".to_string(),
            descriptor: "J".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(bitset_ctx);
    registry.natives_mut().register(
        "java/util/BitSet",
        "<init>",
        "(I)V",
        native_bitset_init_with_size,
    );
    registry
        .natives_mut()
        .register("java/util/BitSet", "<init>", "()V", native_bitset_init);
    registry
        .natives_mut()
        .register("java/util/BitSet", "set", "(I)V", native_bitset_set);
    registry.natives_mut().register(
        "java/util/BitSet",
        "cardinality",
        "()I",
        native_bitset_cardinality,
    );

    // java/util/function/Function — synthetic interface (needed so ClassNotFound doesn't fire)
    let function_iface_ctx = ClassContext {
        class_name: "java/util/function/Function".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(function_iface_ctx);

    // Function.identity() → returns a duke/util/IdentityFunction proxy
    let identity_fn_ctx = ClassContext {
        class_name: "duke/util/IdentityFunction".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec!["java/util/function/Function".to_string()],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(identity_fn_ctx);
    registry.natives_mut().register(
        "java/util/function/Function",
        "identity",
        "()Ljava/util/function/Function;",
        native_function_identity,
    );
    registry.natives_mut().register(
        "duke/util/IdentityFunction",
        "apply",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_identity_function_apply,
    );

    // ---- Phase 117 ----

    // HashMap.remove(Object, Object) -> boolean — conditional remove
    registry.natives_mut().register(
        "java/util/HashMap",
        "remove",
        "(Ljava/lang/Object;Ljava/lang/Object;)Z",
        native_hashmap_remove_key_value,
    );

    // HashMap.replace(Object, Object) -> Object — replace value if key present
    registry.natives_mut().register(
        "java/util/HashMap",
        "replace",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        native_hashmap_replace,
    );

    // Integer.sum(int, int) -> int — static utility
    registry
        .natives_mut()
        .register("java/lang/Integer", "sum", "(II)I", native_integer_sum);

    // Stream.concat(Stream, Stream) -> Stream — static method
    registry.natives_mut().register(
        "duke/util/Stream",
        "concat",
        "(Ljava/util/stream/Stream;Ljava/util/stream/Stream;)Ljava/util/stream/Stream;",
        native_stream_concat,
    );

    // IntStream.mapToObj(IntFunction) -> Stream
    registry.natives_mut().register_callback(
        "duke/util/IntStream",
        "mapToObj",
        "(Ljava/util/function/IntFunction;)Ljava/util/stream/Stream;",
        native_int_stream_map_to_obj,
    );

    // Optional.map(Function) -> Optional
    registry.natives_mut().register_callback(
        "duke/util/Optional",
        "map",
        "(Ljava/util/function/Function;)Ljava/util/Optional;",
        native_optional_map,
    );

    // java/util/UnmodifiableSet — read ops reuse HashSet handlers; writes throw
    let unmod_set_ctx = ClassContext {
        class_name: "java/util/UnmodifiableSet".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: vec![
            "java/util/Set".to_string(),
            "java/util/Collection".to_string(),
        ],
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(unmod_set_ctx);
    // Read operations — reuse HashSet handlers (same field layout)
    for (method, desc, handler) in [
        ("size", "()I", native_hashset_size as NativeHandler),
        ("contains", "(Ljava/lang/Object;)Z", native_hashset_contains),
        ("isEmpty", "()Z", native_hashset_is_empty),
        (
            "iterator",
            "()Ljava/util/Iterator;",
            native_hashset_iterator,
        ),
    ] {
        registry
            .natives_mut()
            .register("java/util/UnmodifiableSet", method, desc, handler);
    }
    // Mutation operations — throw UnsupportedOperationException
    for (method, desc) in [
        ("add", "(Ljava/lang/Object;)Z"),
        ("remove", "(Ljava/lang/Object;)Z"),
        ("clear", "()V"),
    ] {
        registry.natives_mut().register(
            "java/util/UnmodifiableSet",
            method,
            desc,
            native_unmodifiable_list_mutation,
        );
    }

    // --- commons-lang3 canary natives ---
    // Character.toTitleCase(C)C and (I)I — simple titlecase mapping used by
    // StringUtils.capitalize.
    registry.natives_mut().register(
        "java/lang/Character",
        "toTitleCase",
        "(C)C",
        native_char_to_titlecase,
    );
    registry.natives_mut().register(
        "java/lang/Character",
        "toTitleCase",
        "(I)I",
        native_char_to_titlecase,
    );
    // Character.charCount(I)I — used by StringUtils.capitalize code-point walk.
    registry.natives_mut().register(
        "java/lang/Character",
        "charCount",
        "(I)I",
        native_char_char_count,
    );
    // String.<init>([III)V — new String(int[] codePoints, offset, count).
    registry.natives_mut().register(
        "java/lang/String",
        "<init>",
        "([III)V",
        native_string_init_code_points,
    );
    // java/lang/reflect/Array — newInstance/getLength/set, used by
    // commons-lang3 ArrayUtils array-growth helpers.
    registry.natives_mut().register(
        "java/lang/reflect/Array",
        "newInstance",
        "(Ljava/lang/Class;I)Ljava/lang/Object;",
        native_reflect_array_new_instance,
    );
    registry.natives_mut().register(
        "java/lang/reflect/Array",
        "getLength",
        "(Ljava/lang/Object;)I",
        native_reflect_array_get_length,
    );
    registry.natives_mut().register(
        "java/lang/reflect/Array",
        "set",
        "(Ljava/lang/Object;ILjava/lang/Object;)V",
        native_reflect_array_set,
    );
    // Class.getComponentType()Ljava/lang/Class; — array element type mirror.
    registry.natives_mut().register(
        "java/lang/Class",
        "getComponentType",
        "()Ljava/lang/Class;",
        native_class_get_component_type,
    );
    // Arrays.setAll(Object[], IntFunction)V — element-wise generator fill.
    registry.natives_mut().register_callback(
        "java/util/Arrays",
        "setAll",
        "([Ljava/lang/Object;Ljava/util/function/IntFunction;)V",
        native_arrays_set_all_object,
    );

    // ---- gson canary: synthetic marker interfaces / reflection support ----
    // java/lang/reflect/Type — marker interface implemented by java/lang/Class.
    // gson's TypeToken.<init> does `checkcast java/lang/reflect/Type` on a
    // Class literal; register the interface so the cast succeeds.
    let type_iface_ctx = ClassContext {
        class_name: "java/lang/reflect/Type".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(type_iface_ctx);

    // java/lang/ThreadLocal — synthetic single-slot holder (field 0 = value).
    // The real JDK ThreadLocal reads Thread.threadLocals, absent from Duke's
    // synthetic Thread stub; gson's Gson.getAdapter uses a plain ThreadLocal as
    // a per-call recursion guard, which single-slot storage models faithfully.
    let thread_local_ctx = ClassContext {
        class_name: "java/lang/ThreadLocal".to_string(),
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
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(thread_local_ctx);
    for (method, descriptor, handler) in [
        ("<init>", "()V", native_thread_local_init as NativeHandler),
        ("get", "()Ljava/lang/Object;", native_thread_local_get),
        ("set", "(Ljava/lang/Object;)V", native_thread_local_set),
        ("remove", "()V", native_thread_local_remove),
    ] {
        registry
            .natives_mut()
            .register("java/lang/ThreadLocal", method, descriptor, handler);
    }

    // ┌──────────────────────────────────────────────────────────────────────┐
    // │ java/lang/ref reference objects (Spring Boot ladder / commons-logging │
    // │ LogFactory.getFactory WeakReference.get frontier)                     │
    // └──────────────────────────────────────────────────────────────────────┘
    // Minimal NON-COLLECTING Reference/WeakReference: the referent lives in a
    // single strong-ref slot (field 0 on Reference) so get() round-trips it until
    // clear(); no GC weak semantics. commons-logging caches its own defining
    // ClassLoader in a static WeakReference and only ever reads it back, which a
    // strong slot models faithfully. See native_reference_* in native/java_lang.rs.
    let reference_ctx = ClassContext {
        class_name: "java/lang/ref/Reference".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: vec![FieldEntry {
            name: "referent".to_string(),
            descriptor: "Ljava/lang/Object;".to_string(),
            is_static: false,
        }],
        static_fields: Vec::new(),
        instance_field_count: 1,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(reference_ctx);
    // get()/clear() live on the Reference base; WeakReference inherits them (the
    // referent slot is inherited, so a WeakReference instance carries one field).
    for (method, descriptor, handler) in [
        (
            "get",
            "()Ljava/lang/Object;",
            native_reference_get as NativeHandler,
        ),
        ("clear", "()V", native_reference_clear),
    ] {
        registry
            .natives_mut()
            .register("java/lang/ref/Reference", method, descriptor, handler);
    }

    let weak_reference_ctx = ClassContext {
        class_name: "java/lang/ref/WeakReference".to_string(),
        super_class: Some("java/lang/ref/Reference".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(weak_reference_ctx);
    // Only the single-arg WeakReference(referent) constructor is exercised by the
    // ladder (commons-logging's thisClassLoaderRef static initialiser).
    registry.natives_mut().register(
        "java/lang/ref/WeakReference",
        "<init>",
        "(Ljava/lang/Object;)V",
        native_reference_init,
    );

    // java/lang/Class.isAssignableFrom — hierarchy-walking reflection predicate
    // (gson uses it while resolving type adapters).
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "isAssignableFrom",
        "(Ljava/lang/Class;)Z",
        native_class_is_assignable_from,
    );

    // java/lang/Class reflection predicates used by gson's ReflectionHelper to
    // classify the raw type before choosing an instantiation strategy.
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getModifiers",
        "()I",
        native_class_get_modifiers,
    );
    registry.natives_mut().register(
        "java/lang/Class",
        "isAnonymousClass",
        "()Z",
        native_class_is_anonymous_class,
    );
    registry.natives_mut().register(
        "java/lang/Class",
        "isLocalClass",
        "()Z",
        native_class_is_local_class,
    );
    registry
        .natives_mut()
        .register("java/lang/Class", "isRecord", "()Z", native_class_is_record);
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "isInterface",
        "()Z",
        native_class_is_interface,
    );
    registry.natives_mut().register(
        "java/lang/Class",
        "isPrimitive",
        "()Z",
        native_class_is_primitive,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "getGenericSuperclass",
        "()Ljava/lang/reflect/Type;",
        native_class_get_generic_superclass,
    );
    registry.natives_mut().register_callback(
        "java/lang/Class",
        "cast",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
        native_class_cast,
    );

    // java/lang/reflect/Field.getModifiers — gson reads these to skip
    // static/transient fields when building reflective adapters.
    registry.natives_mut().register_callback(
        "java/lang/reflect/Field",
        "getModifiers",
        "()I",
        native_reflect_field_get_modifiers,
    );
    registry.natives_mut().register(
        "java/lang/reflect/Field",
        "isSynthetic",
        "()Z",
        native_reflect_field_is_synthetic,
    );
    registry.natives_mut().register_callback(
        "java/lang/reflect/Field",
        "getGenericType",
        "()Ljava/lang/reflect/Type;",
        native_reflect_field_get_generic_type,
    );

    // java/lang/StringBuffer.append(char) — the char-aware append variant was
    // missing (the int variant would render the code point as a decimal string).
    // Shares StringBuilder's char handler; both back onto `string_value`.
    registry.natives_mut().register(
        "java/lang/StringBuffer",
        "append",
        "(C)Ljava/lang/StringBuffer;",
        native_sb_append_char,
    );
    // StringBuffer.append(CharSequence, int, int) — subrange append used by
    // gson's JsonWriter string escaping. Shares StringBuilder's range handler.
    registry.natives_mut().register(
        "java/lang/StringBuffer",
        "append",
        "(Ljava/lang/CharSequence;II)Ljava/lang/StringBuffer;",
        native_sb_append_charsequence_range,
    );

    // java/util/Objects.checkFromIndexSize — subrange bounds check used by gson's
    // string escaping paths.
    registry.natives_mut().register(
        "java/util/Objects",
        "checkFromIndexSize",
        "(III)I",
        native_objects_check_from_index_size,
    );

    // java/lang/String.getChars — bulk char copy used by gson's JsonWriter.
    registry.natives_mut().register(
        "java/lang/String",
        "getChars",
        "(II[CI)V",
        native_string_get_chars,
    );
    // java/lang/String.<init>(char[], int, int) — subrange constructor used by
    // gson's JsonReader.
    registry.natives_mut().register(
        "java/lang/String",
        "<init>",
        "([CII)V",
        native_string_init_from_chars_range,
    );

    // java/util/Map.of 9-pair overload — gson's Primitives maps the 9 primitive
    // types to their wrappers. native_map_of already handles arbitrary pair counts.
    {
        let obj = "Ljava/lang/Object;";
        let map_of_9 = format!("({})Ljava/util/Map;", obj.repeat(18));
        registry
            .natives_mut()
            .register("java/util/Map", "of", &map_of_9, native_map_of);
    }

    // java/util/HashMap.<init>(Map) — copy constructor used by gson.
    registry.natives_mut().register(
        "java/util/HashMap",
        "<init>",
        "(Ljava/util/Map;)V",
        native_hashmap_init_map,
    );

    // sun/misc/Unsafe — synthetic shadow so the real JDK Unsafe.<clinit> (which
    // reaches for fields Duke cannot resolve) never runs. gson's UnsafeAllocator
    // reflectively reads the static `theUnsafe` field and invokes
    // `allocateInstance(Class)` to construct POJOs lacking a no-arg constructor, so
    // both members must be reflectively visible: `theUnsafe` is a populated static
    // field, and `allocateInstance` is a native-backed MethodEntry.
    let unsafe_instance_ref = heap.allocate("sun/misc/Unsafe".to_string(), 0);
    let unsafe_ctx = ClassContext {
        class_name: "sun/misc/Unsafe".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: vec![crate::context::MethodEntry {
            name: "allocateInstance".to_string(),
            descriptor: "(Ljava/lang/Class;)Ljava/lang/Object;".to_string(),
            is_public: true,
            is_static: false,
            is_native: true,
            is_abstract: false,
            instructions: std::sync::Arc::new([]),
            max_stack: 0,
            max_locals: 0,
            exception_table: Vec::new(),
            pc_to_idx: std::sync::Arc::new(std::collections::HashMap::new()),
            line_number_table: Vec::new(),
            source_file: None,
        }],
        fields: vec![FieldEntry {
            name: "theUnsafe".to_string(),
            descriptor: "Lsun/misc/Unsafe;".to_string(),
            is_static: true,
        }],
        static_fields: vec![Slot::Reference(Some(unsafe_instance_ref))],
        instance_field_count: 0,
        interfaces: Vec::new(),
        bootstrap_methods: Vec::new(),
        load_source: ClassLoadSource::Synthetic,
    };
    registry.register(unsafe_ctx);
    registry.natives_mut().register_callback(
        "sun/misc/Unsafe",
        "allocateInstance",
        "(Ljava/lang/Class;)Ljava/lang/Object;",
        native_unsafe_allocate_instance,
    );

    // ─── java.lang.Module (minimal unnamed-module model) ─────────────────────
    // Intercept `Class.getModule()` and the `Module` accessors the real JDK
    // `ServiceLoader` reaches under `real_jdk_shadow`. Every class lives in the
    // shared unnamed module of the system class loader; `isNamed()==false` lets
    // `ServiceLoader.checkCaller` skip the module-layer `uses` check. Handlers in
    // `native/java_lang.rs`.
    registry.natives_mut().register(
        "java/lang/Class",
        "getModule",
        "()Ljava/lang/Module;",
        native_class_get_module,
    );
    registry
        .natives_mut()
        .register("java/lang/Module", "isNamed", "()Z", native_module_is_named);
    registry.natives_mut().register(
        "java/lang/Module",
        "getName",
        "()Ljava/lang/String;",
        native_module_get_name,
    );
    registry.natives_mut().register(
        "java/lang/Module",
        "canUse",
        "(Ljava/lang/Class;)Z",
        native_module_can_use,
    );
    // jdk/internal/reflect/Reflection.getClassAccessFlags — real
    // `Reflection.verifyMemberAccess` (reached from `ServiceLoader` after the
    // module check) reads the declaring class's access flags to test public access.
    registry.natives_mut().register_callback(
        "jdk/internal/reflect/Reflection",
        "getClassAccessFlags",
        "(Ljava/lang/Class;)I",
        native_reflection_get_class_access_flags,
    );

    // ─── java.lang.invoke / SharedSecrets foundation ─────────────────────────
    // Natives demanded by the real `FileOutputStream.<clinit>` chain under
    // `DUKE_REAL_JDK=1`. The chain routes through `MethodHandles.Lookup`
    // initialization, which forces `<clinit>` of the target class via
    // `Unsafe.ensureClassInitialized`. `Unsafe` stays on `KEEP_SYNTHETIC`, so its
    // methods are dispatched to this native (not shadowed bytecode); the native
    // honors the real contract by driving the class through its real `<clinit>`.
    // Handlers in `native/jdk_internal.rs`.
    registry.natives_mut().register_callback(
        "jdk/internal/misc/Unsafe",
        "ensureClassInitialized",
        "(Ljava/lang/Class;)V",
        native_unsafe_ensure_class_initialized,
    );
    // `FileDescriptor.<clinit>` and `FileOutputStream.<clinit>` (forced by
    // `ensureClassInitialized`) each open with the native `initIDs()V`. HotSpot uses
    // it only to cache jfieldIDs; Duke resolves fields positionally, so it is a
    // no-op. The rest of each `<clinit>` runs as real bytecode.
    registry.natives_mut().register(
        "java/io/FileDescriptor",
        "initIDs",
        "()V",
        native_io_init_ids_noop,
    );
    // Path-based `new FileOutputStream(path)` constructs a `java/io/File`, whose
    // `File.<clinit>` reaches `UnixFileSystem.<clinit>@0 invokestatic initIDs:()V`.
    // Same story: HotSpot caches jfieldIDs there; Duke resolves positionally, so a
    // no-op. This unblocks the `File`/`FileSystem` layer for the path-based stream
    // chain (the stdout/stderr descriptor path does not touch it).
    registry.natives_mut().register(
        "java/io/UnixFileSystem",
        "initIDs",
        "()V",
        native_io_init_ids_noop,
    );
    // `FileOutputStream.initIDs` additionally installs the placeholder
    // `SharedSecrets.javaLangAccess` so the later `FileOutputStream.write(...)` ->
    // `Blocker.<clinit>` boot-sanity check ("JavaLangAccess not setup") passes. This
    // is the guaranteed pre-write seam: `FileOutputStream.<clinit>` already loaded
    // `SharedSecrets` (at its `getJavaIOFileDescriptorAccess()` call) and completes
    // before any instance write reaches `Blocker`.
    registry.natives_mut().register_callback(
        "java/io/FileOutputStream",
        "initIDs",
        "()V",
        native_file_output_stream_init_ids,
    );
    // The real-layout `FileOutputStream.write(byte[])` leaf: resolves the OS
    // descriptor from `this.fd.fd` and routes stdout (1)/stderr (2). `append` (arg 4)
    // is ignored for the standard descriptors, which are never opened in append mode.
    registry.natives_mut().register_callback(
        "java/io/FileOutputStream",
        "writeBytes",
        "([BIIZ)V",
        native_real_file_output_stream_write_bytes,
    );
    // `FileOutputStream.write(...)` brackets its native I/O in `Blocker.begin()`,
    // which — because Duke seeds `VM.isBooted() == true` — reaches
    // `JavaLangAccess.currentCarrierThread()` (dispatched on the placeholder JLA
    // installed by `FileOutputStream.initIDs`). Duke has no virtual threads, so the
    // current platform thread is its own carrier: return `Thread.currentThread()`.
    // It is not a `jdk/internal/misc/CarrierThread`, so `Blocker.begin()` takes the
    // `-1` (no-compensation) branch and `Blocker.end(-1)` is a no-op — the correct
    // behavior for a plain platform thread.
    registry.natives_mut().register(
        "jdk/internal/access/JavaLangAccess",
        "currentCarrierThread",
        "()Ljava/lang/Thread;",
        native_thread_current_thread,
    );
    // The real `FileDescriptor(int)` constructor seeds `handle`/`append` from these
    // natives. `getHandle` is -1 on unix (Windows-only concept); `getAppend` is
    // false for the standard descriptors built in `<clinit>`.
    registry.natives_mut().register(
        "java/io/FileDescriptor",
        "getHandle",
        "(I)J",
        native_file_descriptor_get_handle,
    );
    registry.natives_mut().register(
        "java/io/FileDescriptor",
        "getAppend",
        "(I)Z",
        native_file_descriptor_get_append,
    );
}

// ─── Phase 88 natives ────────────────────────────────────────────────────────

/// `BitSet.<init>(int)V` — size hint ignored; initialise bits to 0.
pub fn native_bitset_init_with_size(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Long(0);
    Ok(None)
}

/// `BitSet.<init>()V`
pub fn native_bitset_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    heap.get_mut(this_ref)?.fields[0] = Slot::Long(0);
    Ok(None)
}

/// `BitSet.set(int)V` — set bit at position n.
pub fn native_bitset_set(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let n = match args.get(1) {
        Some(Slot::Int(v)) => *v,
        _ => {
            return Err(Error::TypeMismatch {
                expected: "int",
                got: "other",
            });
        }
    };
    let bits = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(b)) => *b,
        _ => 0i64,
    };
    let new_bits = bits | (1i64 << (n.cast_unsigned() & 63));
    heap.get_mut(this_ref)?.fields[0] = Slot::Long(new_bits);
    Ok(None)
}

/// `BitSet.cardinality()I` — number of set bits.
pub fn native_bitset_cardinality(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let bits = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(b)) => *b,
        _ => 0i64,
    };
    Ok(Some(Slot::Int(bits.count_ones().cast_signed())))
}

/// `Function.identity()Ljava/util/function/Function;` — returns a `duke/util/IdentityFunction` proxy.
#[allow(clippy::unnecessary_wraps)]
pub fn native_function_identity(
    _args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let r = heap.allocate("duke/util/IdentityFunction".to_string(), 0);
    Ok(Some(Slot::Reference(Some(r))))
}

/// `IdentityFunction.apply(Object)Object` — returns its argument unchanged.
#[allow(clippy::unnecessary_wraps)]
pub fn native_identity_function_apply(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    // args[0] = this (the IdentityFunction proxy), args[1] = the element
    Ok(Some(args.get(1).copied().unwrap_or(Slot::Reference(None))))
}

// ─── Phase 100 natives ───────────────────────────────────────────────────────

/// `Collectors.summarizingInt(ToIntFunction)Collector` — returns a `SummarizingIntCollector`.
pub fn native_collectors_summarizing_int(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let fn_slot = args.first().copied().unwrap_or(Slot::Reference(None));
    let r = heap.allocate("duke/util/SummarizingIntCollector".to_string(), 1);
    heap.get_mut(r)?.fields[0] = fn_slot;
    Ok(Some(Slot::Reference(Some(r))))
}

/// `IntSummaryStatistics.getCount()J`
pub fn native_int_summary_stats_get_count(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let count = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(count)))
}

/// `IntSummaryStatistics.getSum()J`
pub fn native_int_summary_stats_get_sum(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let sum = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Long(sum)))
}

/// `IntSummaryStatistics.getMin()I`
pub fn native_int_summary_stats_get_min(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let min = match heap.get(this_ref)?.fields.get(2) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(min)))
}

/// `IntSummaryStatistics.getMax()I`
pub fn native_int_summary_stats_get_max(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let max = match heap.get(this_ref)?.fields.get(3) {
        Some(Slot::Int(v)) => *v,
        _ => 0,
    };
    Ok(Some(Slot::Int(max)))
}

/// `IntSummaryStatistics.getAverage()D`
pub fn native_int_summary_stats_get_average(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
) -> Result<Option<Slot>> {
    let this_ref = extract_ref_arg(args, 0)?;
    let count = match heap.get(this_ref)?.fields.first() {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    let sum = match heap.get(this_ref)?.fields.get(1) {
        Some(Slot::Long(v)) => *v,
        _ => 0,
    };
    #[allow(clippy::cast_precision_loss)]
    let avg = if count == 0 {
        0.0
    } else {
        sum as f64 / count as f64
    };
    Ok(Some(Slot::Double(avg)))
}

// ---------------------------------------------------------------------------
// System.out / System.err / System.in construction infrastructure.
//
// Scaffolding for the documented multi-wave migration of `java/lang/System` off
// `KEEP_SYNTHETIC`. Real `java/lang/System.initPhase1()` installs the standard
// streams by calling the native methods `setOut0(PrintStream)`,
// `setErr0(PrintStream)` and `setIn0(InputStream)`, which store their argument
// into the `out`/`err`/`in` static fields (bypassing `final`).
//
// Statics live in `ClassContext::static_fields` inside the `ClassRegistry`; a
// native handler cannot reach the registry directly, but a *callback* native
// receives `&mut dyn CallbackOps`, whose `write_static_field` resolves the slot
// by name and stores into `registry.get_mut(class).static_fields[idx]` (the same
// store `putstatic` performs). All three natives and the bootstrap seeder route
// through the single `set_system_stream` helper below, so the store path is
// genuine, live, and exercised on every VM startup. `System` STAYS synthetic
// this wave — these natives are only invoked once `initPhase1` runs a future
// wave, but the shared helper is proven live via bootstrap seeding + unit tests.
// ---------------------------------------------------------------------------

/// Store `stream_ref` into the named `java/lang/System` stream static field
/// (`out`, `err`, or `in`), resolving the slot by name through the registry's
/// static-field store. This is the ONE code path shared by the bootstrap seeder
/// and the `setOut0`/`setErr0`/`setIn0` natives.
///
/// # Errors
/// Returns an error if `java/lang/System` or the named field is not registered.
fn set_system_stream(ops: &mut dyn CallbackOps, field_name: &str, stream_ref: Slot) -> Result<()> {
    ops.write_static_field("java/lang/System", field_name, stream_ref)
}

/// Minimal [`CallbackOps`] adapter exposing only the registry-backed static
/// store, so `bootstrap_stdlib` can seed `System.out`/`err` through the exact
/// same [`set_system_stream`] path the natives use (instead of an inline
/// `static_fields` assignment). Every other callback surface is unused at seed
/// time.
struct SystemSeedOps<'a> {
    registry: &'a mut ClassRegistry,
}

impl CallbackOps for SystemSeedOps<'_> {
    fn invoke(
        &mut self,
        _heap: &mut duke_gc::Heap,
        _output: &mut dyn Write,
        _class: &str,
        _method: &str,
        _descriptor: &str,
        _args: Vec<Slot>,
    ) -> Result<Option<Slot>> {
        // Stream seeding never calls back into Java bytecode.
        Ok(None)
    }

    fn ensure_loaded(&mut self, _class: &str) -> Result<()> {
        Ok(())
    }

    fn inspect_class(&mut self, _class: &str) -> Result<ReflectedClassInfo> {
        Err(Error::Unimplemented {
            mnemonic: "SystemSeedOps::inspect_class",
        })
    }

    fn write_static_field(&mut self, class: &str, field_name: &str, value: Slot) -> Result<()> {
        let slot = static_field_idx(self.registry.get(class)?, field_name)?;
        self.registry.get_mut(class)?.static_fields[slot] = value;
        Ok(())
    }
}

/// Seed `System.out` and `System.err` with the pre-allocated `PrintStream` refs by
/// driving the shared [`set_system_stream`] helper through [`SystemSeedOps`].
///
/// # Panics
/// Panics only if `java/lang/System` was not registered with `out`/`err` static
/// fields immediately before this call — a bootstrap invariant.
fn seed_system_streams(registry: &mut ClassRegistry, out_ref: u64, err_ref: u64) {
    let mut ops = SystemSeedOps { registry };
    set_system_stream(&mut ops, "out", Slot::Reference(Some(out_ref)))
        .expect("System.out static field must exist for bootstrap seeding");
    set_system_stream(&mut ops, "err", Slot::Reference(Some(err_ref)))
        .expect("System.err static field must exist for bootstrap seeding");
}

/// Register the `setOut0`/`setErr0`/`setIn0` static natives on `java/lang/System`.
fn register_system_stream_natives(registry: &mut ClassRegistry) {
    registry.natives_mut().register_callback(
        "java/lang/System",
        "setOut0",
        "(Ljava/io/PrintStream;)V",
        native_system_set_out0,
    );
    registry.natives_mut().register_callback(
        "java/lang/System",
        "setErr0",
        "(Ljava/io/PrintStream;)V",
        native_system_set_err0,
    );
    registry.natives_mut().register_callback(
        "java/lang/System",
        "setIn0",
        "(Ljava/io/InputStream;)V",
        native_system_set_in0,
    );
}

/// Shared body for the `setOut0`/`setErr0`/`setIn0` static natives: store the
/// single stream argument into the named `java/lang/System` static field.
fn system_set_stream_native(
    field_name: &str,
    args: &[Slot],
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    let stream_ref = args.first().copied().ok_or(Error::TypeMismatch {
        expected: "Reference",
        got: "missing argument",
    })?;
    set_system_stream(ops, field_name, stream_ref)?;
    Ok(None)
}

/// `java/lang/System.setOut0(Ljava/io/PrintStream;)V` — install `System.out`.
fn native_system_set_out0(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    system_set_stream_native("out", args, ops)
}

/// `java/lang/System.setErr0(Ljava/io/PrintStream;)V` — install `System.err`.
fn native_system_set_err0(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    system_set_stream_native("err", args, ops)
}

/// `java/lang/System.setIn0(Ljava/io/InputStream;)V` — install `System.in`.
fn native_system_set_in0(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
    _control: &mut NativeControl,
    ops: &mut dyn CallbackOps,
) -> Result<Option<Slot>> {
    system_set_stream_native("in", args, ops)
}

#[cfg(test)]
mod system_stream_seed_tests {
    use super::*;
    use duke_gc::Heap;

    /// Read a `java/lang/System` stream static field by name.
    fn system_stream(registry: &ClassRegistry, field: &str) -> Slot {
        let ctx = registry.get("java/lang/System").expect("System registered");
        let idx = static_field_idx(ctx, field).expect("stream field exists");
        ctx.static_fields[idx]
    }

    #[test]
    fn bootstrap_seeds_out_and_err_via_shared_helper() {
        let mut registry = ClassRegistry::new();
        let mut heap = Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        // out/err are seeded with live PrintStream refs through set_system_stream;
        // in starts null (reserved for the future real-layout migration wave).
        assert!(matches!(
            system_stream(&registry, "out"),
            Slot::Reference(Some(_))
        ));
        assert!(matches!(
            system_stream(&registry, "err"),
            Slot::Reference(Some(_))
        ));
        assert_eq!(system_stream(&registry, "in"), Slot::Reference(None));

        // out and err are distinct PrintStream instances (byte-identical layout).
        assert_ne!(
            system_stream(&registry, "out"),
            system_stream(&registry, "err")
        );
    }

    #[test]
    fn set_out0_native_updates_system_out_static() {
        let mut registry = ClassRegistry::new();
        let mut heap = Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        let original_err = system_stream(&registry, "err");
        let new_ps = heap.allocate("java/io/PrintStream".to_string(), 1);
        let args = [Slot::Reference(Some(new_ps))];
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        {
            let mut ops = SystemSeedOps {
                registry: &mut registry,
            };
            native_system_set_out0(&args, &mut heap, &mut out, &mut control, &mut ops)
                .expect("setOut0 stores the new stream");
        }

        assert_eq!(
            system_stream(&registry, "out"),
            Slot::Reference(Some(new_ps))
        );
        // err is untouched by setOut0.
        assert_eq!(system_stream(&registry, "err"), original_err);
    }

    #[test]
    fn set_err0_native_updates_system_err_static() {
        let mut registry = ClassRegistry::new();
        let mut heap = Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        let new_ps = heap.allocate("java/io/PrintStream".to_string(), 1);
        let args = [Slot::Reference(Some(new_ps))];
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        {
            let mut ops = SystemSeedOps {
                registry: &mut registry,
            };
            native_system_set_err0(&args, &mut heap, &mut out, &mut control, &mut ops)
                .expect("setErr0 stores the new stream");
        }

        assert_eq!(
            system_stream(&registry, "err"),
            Slot::Reference(Some(new_ps))
        );
    }

    #[test]
    fn set_in0_native_updates_system_in_static() {
        let mut registry = ClassRegistry::new();
        let mut heap = Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        assert_eq!(system_stream(&registry, "in"), Slot::Reference(None));
        let stdin_obj = heap.allocate("java/io/InputStream".to_string(), 0);
        let args = [Slot::Reference(Some(stdin_obj))];
        let mut out = Vec::new();
        let mut control = NativeControl::default();
        {
            let mut ops = SystemSeedOps {
                registry: &mut registry,
            };
            native_system_set_in0(&args, &mut heap, &mut out, &mut control, &mut ops)
                .expect("setIn0 stores the new stream");
        }

        assert_eq!(
            system_stream(&registry, "in"),
            Slot::Reference(Some(stdin_obj))
        );
    }

    #[test]
    fn set_system_stream_helper_errors_on_unknown_field() {
        let mut registry = ClassRegistry::new();
        let mut heap = Heap::new();
        bootstrap_stdlib(&mut registry, &mut heap);

        let mut ops = SystemSeedOps {
            registry: &mut registry,
        };
        let err = set_system_stream(&mut ops, "nonexistent", Slot::Reference(None)).unwrap_err();
        assert!(matches!(err, Error::InvalidFieldref { .. }));
    }
}
