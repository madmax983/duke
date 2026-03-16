use crate::ClassContext;
use crate::ClassRegistry;
use crate::FieldEntry;
use duke_runtime::{Slot, VmError, VmResult};
use std::collections::HashMap;
use std::io::Write;
///
/// Arguments:
/// - `&[Slot]`: method arguments (including `this` in slot 0 for instance methods)
/// - `&mut Heap`: the object heap for reading/writing objects
/// - `&mut dyn Write`: output sink (stdout in production, Vec<u8> in tests)
pub type NativeHandler = fn(&[Slot], &mut duke_gc::Heap, &mut dyn Write) -> VmResult<Option<Slot>>;
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
///
/// Defined separately so function signatures that accept this parameter avoid the
/// `clippy::type_complexity` lint.
///
/// `pub` so that external crates can write their own [`CallbackNativeHandler`]
/// implementations.  If external registration is not needed, consider
/// narrowing to `pub(crate)`.
pub type InvokeFn<'a> = dyn FnMut(&mut duke_gc::Heap, &mut dyn Write, &str, &str, &str, Vec<Slot>) -> VmResult<Option<Slot>>
    + 'a;
#[derive(Copy, Clone, Debug)]
pub enum HandlerKind {
    Simple(NativeHandler),
    Callback(CallbackNativeHandler),
}
///
/// Maps `"class_name\x00method_name\x00descriptor"` to a handler kind.
pub struct NativeRegistry {
    handlers: HashMap<String, HandlerKind>,
}
///
/// Using NUL as separator avoids ambiguity (JVM identifiers cannot contain NUL)
/// and reduces the three allocations previously required per lookup to one.
#[inline]
fn make_key(class: &str, method: &str, desc: &str) -> String {
    let mut key = String::with_capacity(class.len() + method.len() + desc.len() + 2);
    key.push_str(class);
    key.push('\x00');
    key.push_str(method);
    key.push('\x00');
    key.push_str(desc);
    key
}

impl NativeRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    fn insert_handler(&mut self, class: &str, method: &str, descriptor: &str, kind: HandlerKind) {
        self.handlers
            .insert(make_key(class, method, descriptor), kind);
    }
    pub fn register(
        &mut self,
        class: &str,
        method: &str,
        descriptor: &str,
        handler: NativeHandler,
    ) {
        self.insert_handler(class, method, descriptor, HandlerKind::Simple(handler));
    }
    pub fn register_callback(
        &mut self,
        class: &str,
        method: &str,
        descriptor: &str,
        handler: CallbackNativeHandler,
    ) {
        self.insert_handler(class, method, descriptor, HandlerKind::Callback(handler));
    }
    ///
    /// Returns `Some` only for `Simple` handlers. Use [`get_kind`] to handle
    /// `Callback` variants.
    ///
    /// [`get_kind`]: NativeRegistry::get_kind
    #[must_use]
    pub fn get(&self, class: &str, method: &str, descriptor: &str) -> Option<NativeHandler> {
        match self.handlers.get(&make_key(class, method, descriptor))? {
            HandlerKind::Simple(h) => Some(*h),
            HandlerKind::Callback(_) => None,
        }
    }
    #[must_use]
    pub fn get_kind(&self, class: &str, method: &str, descriptor: &str) -> Option<HandlerKind> {
        self.handlers
            .get(&make_key(class, method, descriptor))
            .copied()
    }
}

impl Default for NativeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
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
        Some(Slot::Reference(None)) => return Ok(Some(Slot::Int(0))),
        _ => return Ok(Some(Slot::Int(0))),
    };
    let this_str = heap.get(this_ref)?.string_value.clone();
    let other_str = heap.get(other_ref)?.string_value.clone();
    Ok(Some(Slot::Int(if this_str == other_str { 1 } else { 0 })))
}
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
fn native_object_hashcode(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    match args.first() {
        Some(Slot::Reference(Some(r))) => Ok(Some(Slot::Int(*r as i32))),
        _ => Err(VmError::NullPointerException),
    }
}
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
///
/// No-op stub. Control flow is handled entirely by the bytecode desugaring —
/// addSuppressed only affects what getSuppressed() returns, which is not
/// yet implemented. Suppressed exception is silently dropped.
///
/// Signature: args[0] = this (Throwable), args[1] = suppressed (Throwable)
fn native_throwable_add_suppressed(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _stdout: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    Ok(None)
}
/// args: [this_ref, name_ref, ordinal_int]
fn native_enum_init(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let name_slot = args.get(1).cloned().unwrap_or(Slot::Reference(None));
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
        Some(slot @ Slot::Reference(_)) => Ok(Some(slot.clone())),
        _ => Ok(Some(Slot::Reference(None))),
    }
}
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
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('?'),
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
                && let Some(c) = char::from_u32(*v as u32)
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
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('?'),
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
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let begin = match args.get(1) {
        Some(Slot::Int(v)) => *v as usize,
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    if begin > s.len() {
        return Err(VmError::ArrayIndexOutOfBounds {
            index: begin as i32,
            length: s.len(),
        });
    }
    let sub: String = s.chars().skip(begin).collect();
    let r = heap.allocate_string(sub);
    Ok(Some(Slot::Reference(Some(r))))
}
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
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
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
    if begin > end || end > s.len() {
        return Err(VmError::ArrayIndexOutOfBounds {
            index: end as i32,
            length: s.len(),
        });
    }
    let sub: String = s.chars().skip(begin).take(end - begin).collect();
    let r = heap.allocate_string(sub);
    Ok(Some(Slot::Reference(Some(r))))
}
fn native_string_indexof(
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
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let result = s.find(&target).map_or(-1, |i| i as i32);
    Ok(Some(Slot::Int(result)))
}
fn native_string_contains(
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
    Ok(Some(Slot::Int(if s.contains(&target) { 1 } else { 0 })))
}
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
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    Ok(Some(Slot::Int(if s.is_empty() { 1 } else { 0 })))
}

// Dispatch string constants used by the callback-based sort chain.
const COMPARE_TO_METHOD: &str = "compareTo";
const COMPARE_TO_OBJECT_DESC: &str = "(Ljava/lang/Object;)I";
const SORT_COMPARATOR_DESC: &str = "(Ljava/util/Comparator;)V";
///
/// Used by all boxed-type `compareTo` natives to return a consistent,
/// sign-correct value without relying on `Ordering`'s internal discriminant.
#[inline]
fn ordering_to_int(o: std::cmp::Ordering) -> i32 {
    match o {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}
fn native_string_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    native_string_compareto_object(args, heap, out)
}
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
    Ok(Some(Slot::Int(if s.starts_with(&prefix) { 1 } else { 0 })))
}
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
    Ok(Some(Slot::Int(if s.ends_with(&suffix) { 1 } else { 0 })))
}
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
fn native_integer_intvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = heap.get(this_ref)?.fields[0].clone();
    Ok(Some(val))
}
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
fn native_string_value_of_char(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let val = match args.first() {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('?'),
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
                    match precision {
                        Some(p) => Ok(format!("{:.prec$}", v, prec = p)),
                        None => Ok(format!("{:.6}", v)),
                    }
                }
                'x' => match obj.fields.first() {
                    Some(Slot::Int(v)) => Ok(format!("{:x}", v)),
                    Some(Slot::Long(v)) => Ok(format!("{:x}", v)),
                    _ => Ok("0".to_string()),
                },
                'X' => match obj.fields.first() {
                    Some(Slot::Int(v)) => Ok(format!("{:X}", v)),
                    Some(Slot::Long(v)) => Ok(format!("{:X}", v)),
                    _ => Ok("0".to_string()),
                },
                _ => Ok(String::new()),
            }
        }
        _ => Ok(String::new()),
    }
}
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
        let mut precision: Option<usize> = None;
        if chars[i] == '.' {
            i += 1;
            let mut prec_str = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                prec_str.push(chars[i]);
                i += 1;
            }
            precision = prec_str.parse().ok();
        }

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
                            .cloned()
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
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('?'),
        _ => {
            return Err(VmError::TypeMismatch {
                expected: "Int",
                got: "other",
            });
        }
    };
    let new_char = match args.get(2) {
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('?'),
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
fn native_string_tostring(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    Ok(Some(args.first().cloned().unwrap_or(Slot::Reference(None))))
}

// ---- Math natives ----
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
fn native_long_longvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = heap.get(this_ref)?.fields[0].clone();
    Ok(Some(val))
}
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
fn native_double_doublevalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = heap.get(this_ref)?.fields[0].clone();
    Ok(Some(val))
}

// ---- Float class native ----
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
                if let Some(ch) = char::from_u32(*v as u32) {
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
// ---------------------------------------------------------------------------
// StringBuilder natives
// ---------------------------------------------------------------------------
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
        Some(Slot::Reference(None)) => String::new(),
        _ => String::new(),
    };
    let obj = heap.get_mut(this_ref)?;
    obj.string_value = Some(init_str);
    Ok(None)
}
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
        Some(Slot::Reference(None)) => "null".to_string(),
        _ => "null".to_string(),
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push_str(&append_str);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}
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
        buf.push_str(&format!("{val}"));
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}
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
        buf.push_str(&format!("{val}"));
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}
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
        Some(Slot::Int(v)) => char::from_u32(*v as u32).unwrap_or('\0'),
        _ => '\0',
    };
    let obj = heap.get_mut(this_ref)?;
    if let Some(ref mut buf) = obj.string_value {
        buf.push(val);
    }
    Ok(Some(Slot::Reference(Some(this_ref))))
}
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
    Ok(Some(Slot::Int(len as i32)))
}

// Character natives
// ---------------------------------------------------------------------------
fn slot_to_char(slot: &Slot) -> VmResult<char> {
    match slot {
        Slot::Int(v) => Ok(char::from_u32(*v as u32).unwrap_or('\0')),
        _ => Err(VmError::TypeMismatch {
            expected: "Int (char)",
            got: "other",
        }),
    }
}
fn native_char_is_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_ascii_digit()))))
}
fn native_char_is_letter(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_alphabetic()))))
}
fn native_char_is_whitespace(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_whitespace()))))
}
fn native_char_is_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_uppercase()))))
}
fn native_char_is_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_lowercase()))))
}
fn native_char_to_uppercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    let upper = ch.to_uppercase().next().unwrap_or(ch);
    Ok(Some(Slot::Int(upper as i32)))
}
fn native_char_to_lowercase(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    let lower = ch.to_lowercase().next().unwrap_or(ch);
    Ok(Some(Slot::Int(lower as i32)))
}
fn native_char_is_letter_or_digit(
    args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let ch = slot_to_char(args.first().ok_or(VmError::StackUnderflow)?)?;
    Ok(Some(Slot::Int(i32::from(ch.is_alphanumeric()))))
}
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
fn native_char_charvalue(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = heap.get(this_ref)?.fields[0].clone();
    Ok(Some(val))
}

// ---------------------------------------------------------------------------
// ArrayList natives
// ---------------------------------------------------------------------------
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
fn native_arraylist_add(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let element = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(VmError::NullPointerException), // shouldn't happen; init sets fields[0]=Int(0)
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1))) // boolean true
}
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
    match obj.fields.get(idx + 1) {
        Some(slot) => Ok(Some(slot.clone())),
        None => Err(VmError::JavaException {
            class_name: "java/lang/ArrayIndexOutOfBoundsException".to_string(),
        }),
    }
}
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
        Some(Slot::Int(n)) if *n >= 0 => *n as usize,
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
                for elem in elems.iter_mut() {
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
fn native_arraylist_iter_init(
    _args: &[Slot],
    _heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    Ok(None)
}
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
            Some(slot) => slot.clone(),
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
        *slot = val.clone();
    }
    Ok(None)
}
fn native_arrays_fill_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let arr_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let val = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let obj = heap.get_mut(arr_ref)?;
    for slot in &mut obj.fields {
        *slot = val.clone();
    }
    Ok(None)
}
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
        Some(Slot::Int(n)) if *n >= 0 => *n as usize,
        Some(Slot::Int(n)) => return Err(VmError::NegativeArraySize { size: *n }),
        _ => 0,
    };
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[I".to_string(), new_len);
    let dst = heap.get_mut(dst_ref)?;
    for i in 0..new_len {
        dst.fields[i] = src_fields.get(i).cloned().unwrap_or(Slot::Int(0));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}
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
        Some(Slot::Int(n)) if *n >= 0 => *n as usize,
        Some(Slot::Int(n)) => return Err(VmError::NegativeArraySize { size: *n }),
        _ => 0,
    };
    let src_fields = heap.get(src_ref)?.fields.clone();
    let dst_ref = heap.allocate("[Ljava/lang/Object;".to_string(), new_len);
    let dst = heap.get_mut(dst_ref)?;
    for i in 0..new_len {
        dst.fields[i] = src_fields.get(i).cloned().unwrap_or(Slot::Reference(None));
    }
    Ok(Some(Slot::Reference(Some(dst_ref))))
}
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
/// or by the first field (e.g. intValue) for boxed numerics, or by reference identity.
fn slots_equal(a: &Slot, b: &Slot, heap: &duke_gc::Heap) -> bool {
    match (a, b) {
        (Slot::Reference(None), Slot::Reference(None)) => true,
        (Slot::Reference(Some(ra)), Slot::Reference(Some(rb))) => {
            if ra == rb {
                return true;
            }
            let oa = match heap.get(*ra) {
                Ok(o) => o,
                Err(_) => return false,
            };
            let ob = match heap.get(*rb) {
                Ok(o) => o,
                Err(_) => return false,
            };
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
    let key = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let val = args.get(2).cloned().unwrap_or(Slot::Reference(None));
    // Clone fields to release the immutable borrow before mutating.
    let fields = heap.get(this_ref)?.fields.clone();
    let mut i = 1usize;
    while i + 1 < fields.len() {
        if slots_equal(&fields[i], &key, heap) {
            let old = fields[i + 1].clone();
            heap.get_mut(this_ref)?.fields[i + 1] = val;
            return Ok(Some(old));
        }
        i += 2;
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
fn native_hashmap_get(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let key = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();
    let mut i = 1usize;
    while i + 1 < fields.len() {
        if slots_equal(&fields[i], &key, heap) {
            return Ok(Some(fields[i + 1].clone()));
        }
        i += 2;
    }
    Ok(Some(Slot::Reference(None)))
}
fn native_hashmap_contains_key(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let key = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();
    let mut i = 1usize;
    while i + 1 < fields.len() {
        if slots_equal(&fields[i], &key, heap) {
            return Ok(Some(Slot::Int(1)));
        }
        i += 2;
    }
    Ok(Some(Slot::Int(0)))
}
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
    let key = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();
    let mut i = 1usize;
    while i + 1 < fields.len() {
        if slots_equal(&fields[i], &key, heap) {
            let old_val = fields[i + 1].clone();
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
            return Ok(Some(old_val));
        }
        i += 2;
    }
    Ok(Some(Slot::Reference(None)))
}
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
fn native_hashmap_get_or_default(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let key = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let default = args.get(2).cloned().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();
    let mut i = 1usize;
    while i + 1 < fields.len() {
        if slots_equal(&fields[i], &key, heap) {
            return Ok(Some(fields[i + 1].clone()));
        }
        i += 2;
    }
    Ok(Some(default))
}

// ---------------------------------------------------------------------------
// HashSet natives
// ---------------------------------------------------------------------------
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
    let element = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();
    // fields[0] = size, fields[1..] = elements
    for field in fields.iter().skip(1) {
        if slots_equal(field, &element, heap) {
            return Ok(Some(Slot::Int(0))); // duplicate
        }
    }
    let obj = heap.get_mut(this_ref)?;
    match obj.fields.first_mut() {
        Some(Slot::Int(sz)) => *sz += 1,
        _ => return Err(VmError::NullPointerException),
    }
    obj.fields.push(element);
    Ok(Some(Slot::Int(1)))
}
fn native_hashset_contains(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let element = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();
    for field in fields.iter().skip(1) {
        if slots_equal(field, &element, heap) {
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}
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
    let element = args.get(1).cloned().unwrap_or(Slot::Reference(None));
    let fields = heap.get(this_ref)?.fields.clone();
    #[allow(clippy::needless_range_loop)] // i is used in obj.fields.swap(i, last_idx)
    for i in 1..fields.len() {
        if slots_equal(&fields[i], &element, heap) {
            let obj = heap.get_mut(this_ref)?;
            let last_idx = obj.fields.len() - 1;
            obj.fields.swap(i, last_idx);
            obj.fields.truncate(obj.fields.len() - 1);
            match obj.fields.first_mut() {
                Some(Slot::Int(sz)) => *sz -= 1,
                _ => return Err(VmError::NullPointerException),
            }
            return Ok(Some(Slot::Int(1)));
        }
    }
    Ok(Some(Slot::Int(0)))
}
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
