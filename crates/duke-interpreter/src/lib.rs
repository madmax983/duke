//! Switch-dispatch JVM bytecode interpreter for Duke Phase 4.
//!
//! Executes decoded instruction streams for methods containing integer, long,
//! float, and double arithmetic, control flow, and local variables.  Heap
//! allocation, field access, and method invocation are not yet implemented.

use std::collections::{HashMap, HashSet};
use std::io::Write;

use duke_bytecode::Instruction;
use duke_bytecode::instruction::ArrayType;
use duke_classfile::types::CpEntry;
use duke_loader::ClassLoader;
use duke_runtime::{Frame, Slot, VmError, VmResult};

/// A decoded method ready for execution.
pub struct MethodEntry {
    pub name: String,
    pub descriptor: String,
    pub instructions: Vec<(usize, Instruction)>,
    pub max_stack: u16,
    pub max_locals: u16,
    pub exception_table: Vec<ExceptionEntry>,
}

/// A field declaration extracted from a parsed class.
pub struct FieldEntry {
    pub name: String,
    pub descriptor: String,
    /// True if declared `static`.
    pub is_static: bool,
}

/// A resolved exception table entry for handler dispatch.
///
/// Built from `duke_classfile::types::ExceptionTableEntry` with catch_type
/// resolved from a CP index to a class name string.
pub struct ExceptionEntry {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    /// `None` for catch-all (finally). `Some(class_name)` for typed catches.
    pub catch_type: Option<String>,
}

/// A parsed class with all methods decoded — the unit of execution for Phase 5+.
pub struct ClassContext {
    /// Internal JVM class name (e.g. `"Point"`).
    pub class_name: String,
    /// Superclass name (`None` for `java/lang/Object`).
    pub super_class: Option<String>,
    pub constant_pool: Vec<Option<CpEntry>>,
    pub methods: Vec<MethodEntry>,
    /// All field declarations (static and instance), in class file order.
    pub fields: Vec<FieldEntry>,
    /// Values of static fields, indexed by position among static-only fields.
    pub static_fields: Vec<Slot>,
    /// Number of instance (non-static) fields — used to size heap objects at `new`.
    pub instance_field_count: usize,
}

/// Registry of loaded classes — maps class name to its ClassContext.
///
/// Used by `execute_class` for cross-class method dispatch.
pub struct ClassRegistry {
    classes: HashMap<String, ClassContext>,
    natives: NativeRegistry,
    /// Tracks which classes have had their `<clinit>` run.
    initialized: HashSet<String>,
}

impl ClassRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            natives: NativeRegistry::new(),
            initialized: HashSet::new(),
        }
    }

    /// Check if a class has been initialized (clinit has run).
    #[must_use]
    pub fn is_initialized(&self, name: &str) -> bool {
        self.initialized.contains(name)
    }

    /// Mark a class as initialized.
    pub fn mark_initialized(&mut self, name: &str) {
        self.initialized.insert(name.to_string());
    }

    /// Access the native method registry.
    #[must_use]
    pub fn natives(&self) -> &NativeRegistry {
        &self.natives
    }

    /// Access the native method registry mutably.
    pub fn natives_mut(&mut self) -> &mut NativeRegistry {
        &mut self.natives
    }

    /// Register a pre-built ClassContext.
    pub fn register(&mut self, ctx: ClassContext) {
        self.classes.insert(ctx.class_name.clone(), ctx);
    }

    /// Get a reference to a loaded class.
    ///
    /// # Errors
    /// Returns [`VmError::ClassNotFound`] if the class is not loaded.
    pub fn get(&self, name: &str) -> VmResult<&ClassContext> {
        self.classes
            .get(name)
            .ok_or_else(|| VmError::ClassNotFound {
                name: name.to_string(),
            })
    }

    /// Get a mutable reference to a loaded class.
    ///
    /// # Errors
    /// Returns [`VmError::ClassNotFound`] if the class is not loaded.
    pub fn get_mut(&mut self, name: &str) -> VmResult<&mut ClassContext> {
        self.classes
            .get_mut(name)
            .ok_or_else(|| VmError::ClassNotFound {
                name: name.to_string(),
            })
    }

    /// Ensure a class is loaded. If not already present, loads it via the class
    /// loader, parses it, builds a ClassContext, and registers it.
    ///
    /// Returns `Ok(true)` if loaded, `Ok(false)` if the class could not be found
    /// (soft failure — for classes like `java/lang/Object` that we can't load yet).
    pub fn ensure_loaded(&mut self, name: &str, loader: &dyn ClassLoader) -> VmResult<bool> {
        if self.classes.contains_key(name) {
            return Ok(true);
        }
        let bytes = match loader.find_class(name) {
            Ok(b) => b,
            Err(_) => return Ok(false),
        };
        let cf = match duke_classfile::parse(&bytes) {
            Ok(cf) => cf,
            Err(_) => return Ok(false),
        };
        let ctx = build_class_context(&cf);
        self.classes.insert(name.to_string(), ctx);
        Ok(true)
    }

    /// Check if a class is loaded.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.classes.contains_key(name)
    }
}

impl Default for ClassRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Signature for native method implementations.
///
/// Arguments:
/// - `&[Slot]`: method arguments (including `this` in slot 0 for instance methods)
/// - `&mut Heap`: the object heap for reading/writing objects
/// - `&mut dyn Write`: output sink (stdout in production, Vec<u8> in tests)
pub type NativeHandler = fn(&[Slot], &mut duke_gc::Heap, &mut dyn Write) -> VmResult<Option<Slot>>;

/// Registry of native method implementations.
///
/// Maps `(class_name, method_name, descriptor)` to a Rust function pointer.
pub struct NativeRegistry {
    methods: HashMap<(String, String, String), NativeHandler>,
}

impl NativeRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
        }
    }

    /// Register a native method handler.
    pub fn register(
        &mut self,
        class: &str,
        method: &str,
        descriptor: &str,
        handler: NativeHandler,
    ) {
        self.methods.insert(
            (
                class.to_string(),
                method.to_string(),
                descriptor.to_string(),
            ),
            handler,
        );
    }

    /// Look up a native handler for the given class/method/descriptor.
    #[must_use]
    pub fn get(&self, class: &str, method: &str, descriptor: &str) -> Option<&NativeHandler> {
        self.methods.get(&(
            class.to_string(),
            method.to_string(),
            descriptor.to_string(),
        ))
    }
}

impl Default for NativeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

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
    registry
        .natives_mut()
        .register("java/io/PrintStream", "println", "(J)V", native_println_long);
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
    registry.natives_mut().register(
        "java/io/PrintStream",
        "print",
        "(D)V",
        native_print_double,
    );
    registry.natives_mut().register(
        "java/io/PrintStream",
        "print",
        "(Z)V",
        native_print_boolean,
    );
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

    // java/lang/Throwable extends Object
    let throwable_ctx = ClassContext {
        class_name: "java/lang/Throwable".to_string(),
        super_class: Some("java/lang/Object".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
    };
    registry.register(throwable_ctx);

    // java/lang/Exception extends Throwable
    let exception_ctx = ClassContext {
        class_name: "java/lang/Exception".to_string(),
        super_class: Some("java/lang/Throwable".to_string()),
        constant_pool: Vec::new(),
        methods: Vec::new(),
        fields: Vec::new(),
        static_fields: Vec::new(),
        instance_field_count: 0,
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
    };
    registry.register(rte_ctx);
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
        Some(Slot::Reference(None)) => return Ok(Some(Slot::Int(0))),
        _ => return Ok(Some(Slot::Int(0))),
    };
    let this_str = heap.get(this_ref)?.string_value.clone();
    let other_str = heap.get(other_ref)?.string_value.clone();
    Ok(Some(Slot::Int(if this_str == other_str { 1 } else { 0 })))
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
        Some(Slot::Reference(Some(r))) => Ok(Some(Slot::Int(*r as i32))),
        _ => Err(VmError::NullPointerException),
    }
}

/// Native: `Object.toString()` — returns `ClassName@hexHash`.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn native_object_tostring(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let class_name = heap.get(this_ref)?.class_name.clone();
    let hash = this_ref as i32;
    let s = format!("{class_name}@{hash:x}");
    let r = heap.allocate_string(s);
    Ok(Some(Slot::Reference(Some(r))))
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

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn native_println_object(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    match args.get(1) {
        Some(Slot::Reference(Some(r))) => {
            let obj = heap.get(*r)?;
            if let Some(s) = &obj.string_value {
                writeln!(out, "{s}").ok();
            } else {
                let hash = *r as i32;
                writeln!(out, "{}@{:x}", obj.class_name, hash).ok();
            }
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
            let obj = heap.get(*r)?;
            if let Some(s) = &obj.string_value {
                write!(out, "{s}").ok();
            } else {
                let hash = *r as i32;
                write!(out, "{}@{:x}", obj.class_name, hash).ok();
            }
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
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    Ok(Some(Slot::Int(if s.is_empty() { 1 } else { 0 })))
}

/// Native: `String.compareTo(String)` — lexicographic comparison.
fn native_string_compareto(
    args: &[Slot],
    heap: &mut duke_gc::Heap,
    _out: &mut dyn Write,
) -> VmResult<Option<Slot>> {
    let this_ref = match args.first() {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();
    let other_ref = match args.get(1) {
        Some(Slot::Reference(Some(r))) => *r,
        _ => return Err(VmError::NullPointerException),
    };
    let other = heap
        .get(other_ref)?
        .string_value
        .clone()
        .unwrap_or_default();
    Ok(Some(Slot::Int(s.cmp(&other) as i32)))
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
    Ok(Some(Slot::Int(if s.starts_with(&prefix) { 1 } else { 0 })))
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
    Ok(Some(Slot::Int(if s.ends_with(&suffix) { 1 } else { 0 })))
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
                frame.push(v.clone())?;
                frame.push(v)?;
            }
            Instruction::DupX1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v1.clone())?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::DupX2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v1.clone())?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v2.clone())?;
                frame.push(v1.clone())?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v2.clone())?;
                frame.push(v1.clone())?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                let v4 = frame.pop()?;
                frame.push(v2.clone())?;
                frame.push(v1.clone())?;
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
                frame.push(Slot::Int(v as i8 as i32))?;
            }
            Instruction::I2c => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(v as u16 as i32))?;
            }
            Instruction::I2s => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(v as i16 as i32))?;
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
                let v = fields[idx_val as usize].clone();
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
                let v = fields[idx_val as usize].as_int()? as i8 as i32;
                frame.push(Slot::Int(v))?;
            }
            Instruction::Bastore => {
                let val = frame.pop_int()? as i8 as i32;
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
                let v = fields[idx_val as usize].as_int()? as u16 as i32;
                frame.push(Slot::Int(v))?;
            }
            Instruction::Castore => {
                let val = frame.pop_int()? as u16 as i32;
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
                let v = fields[idx_val as usize].as_int()? as i16 as i32;
                frame.push(Slot::Int(v))?;
            }
            Instruction::Sastore => {
                let val = frame.pop_int()? as i16 as i32;
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
fn ensure_initialized(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    heap: &mut duke_gc::Heap,
    stdout: &mut dyn Write,
    class_name: &str,
) -> VmResult<()> {
    if registry.is_initialized(class_name) {
        return Ok(());
    }
    // Mark as initialized BEFORE running clinit to prevent infinite recursion.
    registry.mark_initialized(class_name);

    // Check if the class has a <clinit> method.
    let has_clinit = {
        match registry.get(class_name) {
            Ok(ctx) => ctx
                .methods
                .iter()
                .any(|m| m.name == "<clinit>" && m.descriptor == "()V"),
            Err(_) => false,
        }
    };

    if has_clinit {
        // Run <clinit> by calling it through execute_class.
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
    }
    Ok(())
}

/// Execute a static method by name within a loaded class context.
///
/// Supports `invokestatic` calls between methods in the same class.
///
/// # Errors
/// Returns [`VmError`] on execution faults or if `method_name`/`descriptor`
/// are not found in `ctx`.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::too_many_lines
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
    ensure_initialized(registry, loader, heap, stdout, class_name)?;

    let mut current_class = class_name.to_string();
    let mut call_stack: Vec<CallFrame> = Vec::new();
    let mut method_idx = entry_idx;
    let mut pc_to_idx: HashMap<usize, usize> = {
        let ctx = registry.get(&current_class)?;
        ctx.methods[method_idx]
            .instructions
            .iter()
            .enumerate()
            .map(|(i, &(pc, _))| (pc, i))
            .collect()
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

    loop {
        let (pc, instr) = {
            let ctx = registry.get(&current_class)?;
            let Some(&(pc, ref instr)) = ctx.methods[method_idx].instructions.get(idx) else {
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
                        frame = caller.frame;
                        method_idx = caller.method_idx;
                        pc_to_idx = caller.pc_to_idx;
                        idx = caller.resume_idx;
                        current_class = caller.class_name;
                        if let Some(v) = ret_val {
                            frame.push(v)?;
                        }
                        continue;
                    }
                }
            }};
        }

        match &instr {
            // ---- invokestatic ----
            Instruction::Invokestatic(cp_idx) => {
                let (callee_class, callee_name, callee_desc) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_methodref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                registry.ensure_loaded(&callee_class, loader)?;
                ensure_initialized(registry, loader, heap, stdout, &callee_class)?;
                let callee_idx = {
                    let ctx = registry.get(&callee_class)?;
                    ctx.methods
                        .iter()
                        .position(|m| m.name == callee_name && m.descriptor == callee_desc)
                };
                match callee_idx {
                    Some(callee_idx) => {
                        let arg_count = parse_arg_count(&callee_desc);
                        let mut callee_args: Vec<Slot> = (0..arg_count)
                            .map(|_| frame.pop())
                            .collect::<VmResult<Vec<_>>>()?;
                        callee_args.reverse();
                        let (callee_pc_to_idx, callee_frame) = {
                            let ctx = registry.get(&callee_class)?;
                            let pci: HashMap<usize, usize> = ctx.methods[callee_idx]
                                .instructions
                                .iter()
                                .enumerate()
                                .map(|(i, &(pc, _))| (pc, i))
                                .collect();
                            let f = Frame::new(
                                usize::from(ctx.methods[callee_idx].max_stack),
                                usize::from(ctx.methods[callee_idx].max_locals),
                                callee_args,
                            )?;
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
                        idx = 0;
                        continue;
                    }
                    None => {
                        // Check native registry before erroring.
                        if let Some(handler) =
                            registry
                                .natives()
                                .get(&callee_class, &callee_name, &callee_desc)
                        {
                            let handler = *handler;
                            let arg_count = parse_arg_count(&callee_desc);
                            let mut native_args: Vec<Slot> = (0..arg_count)
                                .map(|_| frame.pop())
                                .collect::<VmResult<Vec<_>>>()?;
                            native_args.reverse();
                            let result = handler(&native_args, heap, stdout)?;
                            if let Some(val) = result {
                                frame.push(val)?;
                            }
                            idx += 1;
                            continue;
                        }
                        return Err(VmError::MethodNotFound {
                            name: callee_name,
                            descriptor: callee_desc,
                        });
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
                    let ctx = registry.get(&current_class)?;
                    ldc_push(&mut frame, &ctx.constant_pool, cp_idx)?;
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
                    let ctx = registry.get(&current_class)?;
                    ldc_push(&mut frame, &ctx.constant_pool, idx_val)?;
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
                frame.push(v.clone())?;
                frame.push(v)?;
            }
            Instruction::DupX1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v1.clone())?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::DupX2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v1.clone())?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v2.clone())?;
                frame.push(v1.clone())?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X1 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                frame.push(v2.clone())?;
                frame.push(v1.clone())?;
                frame.push(v3)?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            Instruction::Dup2X2 => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                let v3 = frame.pop()?;
                let v4 = frame.pop()?;
                frame.push(v2.clone())?;
                frame.push(v1.clone())?;
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
                frame.push(Slot::Int(v as i8 as i32))?;
            }
            Instruction::I2c => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(v as u16 as i32))?;
            }
            Instruction::I2s => {
                let v = frame.pop_int()?;
                frame.push(Slot::Int(v as i16 as i32))?;
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
                ensure_initialized(registry, loader, heap, stdout, &target_class)?;
                let field_count = registry
                    .get(&target_class)
                    .map(|c| c.instance_field_count)
                    .unwrap_or(0);
                let r = heap.allocate(target_class, field_count);
                frame.push(Slot::Reference(Some(r)))?;
            }

            // ---- Field access ----
            Instruction::Getfield(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let r = frame.pop_ref()?;
                registry.ensure_loaded(&target_class, loader)?;
                let fidx = instance_field_idx(registry.get(&target_class)?, &field_name)?;
                let val = heap.get(r)?.fields[fidx].clone();
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
                let fidx = instance_field_idx(registry.get(&target_class)?, &field_name)?;
                heap.get_mut(r)?.fields[fidx] = val;
            }
            Instruction::Getstatic(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                registry.ensure_loaded(&target_class, loader)?;
                ensure_initialized(registry, loader, heap, stdout, &target_class)?;
                let sidx = static_field_idx(registry.get(&target_class)?, &field_name)?;
                let val = registry.get(&target_class)?.static_fields[sidx].clone();
                frame.push(val)?;
            }
            Instruction::Putstatic(cp_idx) => {
                let (target_class, field_name, _) = {
                    let ctx = registry.get(&current_class)?;
                    resolve_fieldref(&ctx.constant_pool, usize::from(cp_idx.0))?
                };
                let val = frame.pop()?;
                registry.ensure_loaded(&target_class, loader)?;
                ensure_initialized(registry, loader, heap, stdout, &target_class)?;
                let sidx = static_field_idx(registry.get(&target_class)?, &field_name)?;
                registry.get_mut(&target_class)?.static_fields[sidx] = val;
            }

            // ---- Instance method dispatch ----
            //
            // invokespecial and invokevirtual: resolve class+name+descriptor from
            // the Methodref.  Dispatch cross-class via registry; unloadable
            // classes (e.g. java/lang/Object) fall back to no-op.
            Instruction::Invokespecial(cp_idx) | Instruction::Invokevirtual(cp_idx) => {
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
                let loaded = registry.ensure_loaded(&callee_class, loader)?;
                let resolved = if loaded {
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
                        // Check native registry before no-op fallback.
                        if let Some(handler) =
                            registry
                                .natives()
                                .get(&callee_class, &callee_name, &callee_desc)
                        {
                            let handler = *handler;
                            let arg_count = parse_arg_count(&callee_desc);
                            let mut native_args: Vec<Slot> = (0..arg_count)
                                .map(|_| frame.pop())
                                .collect::<VmResult<Vec<_>>>()?;
                            native_args.reverse();
                            let this_slot = frame.pop()?; // pop `this`
                            native_args.insert(0, this_slot);
                            let result = handler(&native_args, heap, stdout)?;
                            if let Some(val) = result {
                                frame.push(val)?;
                            }
                            idx += 1;
                            continue;
                        }
                        // Unloadable or missing — pop args + this and continue.
                        let arg_count = parse_arg_count(&callee_desc);
                        for _ in 0..arg_count {
                            frame.pop()?;
                        }
                        frame.pop()?; // pop `this`
                        idx += 1;
                        continue;
                    }
                };
                let arg_count = parse_arg_count(&callee_desc);
                let mut callee_args: Vec<Slot> = (0..arg_count)
                    .map(|_| frame.pop())
                    .collect::<VmResult<Vec<_>>>()?;
                callee_args.reverse();
                // Pop `this` ref and prepend as locals[0].
                let this_slot = frame.pop()?;
                callee_args.insert(0, this_slot);
                let (callee_pc_to_idx, callee_frame) = {
                    let ctx = registry.get(&dispatch_class)?;
                    let pci: HashMap<usize, usize> = ctx.methods[callee_idx]
                        .instructions
                        .iter()
                        .enumerate()
                        .map(|(i, &(pc, _))| (pc, i))
                        .collect();
                    let f = Frame::new(
                        usize::from(ctx.methods[callee_idx].max_stack),
                        usize::from(ctx.methods[callee_idx].max_locals),
                        callee_args,
                    )?;
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
                let obj = heap.get_mut(r)?;
                if idx_val < 0 || idx_val as usize >= obj.fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: obj.fields.len(),
                    });
                }
                obj.fields[idx_val as usize] = Slot::Int(val);
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
                let obj = heap.get_mut(r)?;
                if idx_val < 0 || idx_val as usize >= obj.fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: obj.fields.len(),
                    });
                }
                obj.fields[idx_val as usize] = Slot::Long(val);
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
                let obj = heap.get_mut(r)?;
                if idx_val < 0 || idx_val as usize >= obj.fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: obj.fields.len(),
                    });
                }
                obj.fields[idx_val as usize] = Slot::Float(val);
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
                let obj = heap.get_mut(r)?;
                if idx_val < 0 || idx_val as usize >= obj.fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: obj.fields.len(),
                    });
                }
                obj.fields[idx_val as usize] = Slot::Double(val);
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
                    fields[idx_val as usize].clone()
                };
                frame.push(v)?;
            }
            Instruction::Aastore => {
                let val = frame.pop()?;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let obj = heap.get_mut(r)?;
                if idx_val < 0 || idx_val as usize >= obj.fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: obj.fields.len(),
                    });
                }
                obj.fields[idx_val as usize] = val;
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
                    fields[idx_val as usize].as_int()? as i8 as i32
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Bastore => {
                let val = frame.pop_int()? as i8 as i32;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let obj = heap.get_mut(r)?;
                if idx_val < 0 || idx_val as usize >= obj.fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: obj.fields.len(),
                    });
                }
                obj.fields[idx_val as usize] = Slot::Int(val);
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
                    fields[idx_val as usize].as_int()? as u16 as i32
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Castore => {
                let val = frame.pop_int()? as u16 as i32;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let obj = heap.get_mut(r)?;
                if idx_val < 0 || idx_val as usize >= obj.fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: obj.fields.len(),
                    });
                }
                obj.fields[idx_val as usize] = Slot::Int(val);
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
                    fields[idx_val as usize].as_int()? as i16 as i32
                };
                frame.push(Slot::Int(v))?;
            }
            Instruction::Sastore => {
                let val = frame.pop_int()? as i16 as i32;
                let idx_val = frame.pop_int()?;
                let r = frame.pop_ref()?;
                let obj = heap.get_mut(r)?;
                if idx_val < 0 || idx_val as usize >= obj.fields.len() {
                    return Err(VmError::ArrayIndexOutOfBounds {
                        index: idx_val,
                        length: obj.fields.len(),
                    });
                }
                obj.fields[idx_val as usize] = Slot::Int(val);
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
                let handler =
                    find_exception_handler(&exc_table, pc, &exc_class_name, registry, loader);
                if let Some(handler_pc) = handler {
                    frame.clear_stack();
                    frame.push(Slot::Reference(Some(exception_ref)))?;
                    idx = *pc_to_idx.get(&(handler_pc as usize)).ok_or(
                        VmError::InvalidBranchTarget {
                            pc: handler_pc as usize,
                        },
                    )?;
                    continue;
                }

                // No handler in current method — unwind call stack.
                loop {
                    match call_stack.pop() {
                        None => {
                            return Err(VmError::JavaException {
                                class_name: exc_class_name,
                            });
                        }
                        Some(caller) => {
                            frame = caller.frame;
                            method_idx = caller.method_idx;
                            pc_to_idx = caller.pc_to_idx;
                            current_class = caller.class_name;

                            // Clone exception table and compute caller_pc before hierarchy check.
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
                                frame.clear_stack();
                                frame.push(Slot::Reference(Some(exception_ref)))?;
                                idx = *pc_to_idx.get(&(handler_pc as usize)).ok_or(
                                    VmError::InvalidBranchTarget {
                                        pc: handler_pc as usize,
                                    },
                                )?;
                                break;
                            }
                            // No handler here either — keep unwinding.
                        }
                    }
                }
                continue;
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
                // Pop args and `this` to determine actual class.
                let mut callee_args: Vec<Slot> = (0..arg_count)
                    .map(|_| frame.pop())
                    .collect::<VmResult<Vec<_>>>()?;
                callee_args.reverse();
                let this_slot = frame.pop()?;
                let actual_class = match &this_slot {
                    Slot::Reference(Some(r)) => heap.get(*r)?.class_name.clone(),
                    _ => callee_class.clone(),
                };
                callee_args.insert(0, this_slot);

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
                            let native = registry
                                .natives()
                                .get(&actual_class, &callee_name, &callee_desc)
                                .or_else(|| {
                                    registry.natives().get(
                                        &callee_class,
                                        &callee_name,
                                        &callee_desc,
                                    )
                                })
                                .copied();
                            if let Some(handler) = native {
                                let result = handler(&callee_args, heap, stdout)?;
                                if let Some(val) = result {
                                    frame.push(val)?;
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

                let (callee_pc_to_idx, callee_frame) = {
                    let ctx = registry.get(&dispatch_class)?;
                    let pci: HashMap<usize, usize> = ctx.methods[callee_idx]
                        .instructions
                        .iter()
                        .enumerate()
                        .map(|(i, &(pc, _))| (pc, i))
                        .collect();
                    let f = Frame::new(
                        usize::from(ctx.methods[callee_idx].max_stack),
                        usize::from(ctx.methods[callee_idx].max_locals),
                        callee_args,
                    )?;
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

/// Saved state of a caller frame suspended during a method call.
struct CallFrame {
    frame: Frame,
    method_idx: usize,
    pc_to_idx: HashMap<usize, usize>,
    resume_idx: usize,
    /// Class that was executing when this frame was pushed.
    class_name: String,
}

/// Build a [`ClassContext`] from a parsed [`ClassFile`].
///
/// Decodes all methods with a Code attribute and extracts field metadata.
/// Methods without Code (abstract, native) are silently skipped.
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
            Some(MethodEntry {
                name,
                descriptor,
                instructions,
                max_stack: code.max_stack,
                max_locals: code.max_locals,
                exception_table,
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

    ClassContext {
        class_name,
        super_class,
        constant_pool: cf.constant_pool.clone(),
        methods,
        fields,
        static_fields: vec![Slot::Int(0); static_count],
        instance_field_count: instance_count,
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
fn is_assignable_from(
    registry: &mut ClassRegistry,
    loader: &dyn ClassLoader,
    from: &str,
    to: &str,
) -> bool {
    if from == to {
        return true;
    }
    if to == "java/lang/Object" {
        return true;
    }
    let mut current = from.to_string();
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(current.clone()) {
            return false; // circular hierarchy — bail
        }
        let _ = registry.ensure_loaded(&current, loader);
        let super_name = match registry.get(&current) {
            Ok(ctx) => ctx.super_class.clone(),
            Err(_) => return false,
        };
        match super_name {
            Some(s) if s == to => return true,
            Some(s) => current = s,
            None => return false,
        }
    }
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
/// Returns (class_name_where_found, method_index) or None.
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
                    Some(s) => current = s.clone(),
                    None => return None,
                }
            }
            Err(_) => return None,
        }
    }
}

/// Resolve a constant pool Methodref to (class_name, method_name, descriptor).
fn resolve_methodref(cp: &[Option<CpEntry>], idx: usize) -> VmResult<(String, String, String)> {
    match cp.get(idx).and_then(|e| e.as_ref()) {
        Some(CpEntry::Methodref {
            class_index,
            name_and_type_index,
        })
        | Some(CpEntry::InterfaceMethodref {
            class_index,
            name_and_type_index,
        }) => {
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
        .find(')')
        .map(|i| &descriptor[1..i])
        .unwrap_or("");
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

/// Resolve a constant pool Fieldref to (class_name, field_name, descriptor).
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

/// Index of a named instance field within ctx.fields (non-static only).
fn instance_field_idx(ctx: &ClassContext, name: &str) -> VmResult<usize> {
    ctx.fields
        .iter()
        .filter(|f| !f.is_static)
        .position(|f| f.name == name)
        .ok_or(VmError::InvalidFieldref { index: 0 })
}

/// Index of a named static field within ctx.static_fields.
fn static_field_idx(ctx: &ClassContext, name: &str) -> VmResult<usize> {
    ctx.fields
        .iter()
        .filter(|f| f.is_static)
        .position(|f| f.name == name)
        .ok_or(VmError::InvalidFieldref { index: 0 })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Unit tests: hand-crafted instruction streams ----

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
}
