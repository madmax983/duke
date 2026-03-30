//! Core execution context types for methods and classes.
//!
//! This module defines the structural components required to execute loaded classes,
//! such as parsed methods, field layouts, and exception handler tables.

use duke_bytecode::Instruction;
use duke_classfile::types::CpEntry;
use duke_runtime::Slot;

/// A decoded method ready for execution.
///
/// This is the engine room of the JVM. It contains not just the raw bytecode
/// instructions, but the pre-computed control flow mapping (`pc_to_idx`) and
/// stack sizing requirements needed to rapidly execute a method invocation.
///
/// # Examples
///
/// ```
/// use duke_interpreter::context::MethodEntry;
/// use std::sync::Arc;
/// use std::collections::HashMap;
///
/// let method = MethodEntry {
///     name: "add".to_string(),
///     descriptor: "(II)I".to_string(),
///     is_public: true,
///     is_static: true,
///     instructions: Arc::new([]),
///     max_stack: 2,
///     max_locals: 2,
///     exception_table: vec![],
///     pc_to_idx: Arc::new(HashMap::new()),
/// };
/// assert_eq!(method.max_stack, 2);
/// ```
pub struct MethodEntry {
    /// The identifier used for method resolution (e.g. `"main"` or `"<init>"`).
    pub name: String,
    /// The signature defining argument types and return type (e.g. `"([Ljava/lang/String;)V"`).
    pub descriptor: String,
    /// Whether Java reflection should treat the method as public.
    pub is_public: bool,
    /// Whether the method is static and therefore takes no implicit `this`.
    pub is_static: bool,
    /// The linear sequence of executable instructions, paired with their original byte offset
    /// in the `.class` file to allow for accurate branch resolution and stack trace generation.
    pub instructions: std::sync::Arc<[(usize, Instruction)]>,
    /// The exact maximum depth the operand stack will reach during execution,
    /// used to pre-allocate memory safely without growing the stack dynamically.
    pub max_stack: u16,
    /// The number of local variable slots required, including arguments and `this`.
    pub max_locals: u16,
    /// A list of active exception handlers that dictate where control flow should jump
    /// if a specific error is thrown within their designated PC bounds.
    pub exception_table: Vec<ExceptionEntry>,
    /// Precomputed PC → instruction-index map, shared cheaply via Arc to ensure
    /// `GOTO` and branch instructions resolve in O(1) time.
    pub pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
}

/// A field declaration extracted from a parsed class.
///
/// This represents the structural definition of a field as declared in the `.class` file,
/// mapping exactly to how the Java compiler saw the field layout. It does not hold
/// runtime values, but rather acts as the blueprint for creating [`duke_runtime::Slot`]s
/// during object allocation.
///
/// # Examples
///
/// ```
/// use duke_interpreter::context::FieldEntry;
///
/// // A blueprint for `private int x;`
/// let field = FieldEntry {
///     name: "x".to_string(),
///     descriptor: "I".to_string(),
///     is_static: false,
/// };
/// assert_eq!(field.name, "x");
/// ```
pub struct FieldEntry {
    /// The exact identifier used in the source code (e.g., `"MAX_VALUE"` or `"counter"`).
    pub name: String,
    /// The JVM type descriptor dictating the field's size and type (e.g., `"I"` for int,
    /// or `"Ljava/lang/String;"` for object references).
    pub descriptor: String,
    /// If `true`, this field belongs to the class itself and its value is stored in
    /// [`ClassContext::static_fields`]. If `false`, it's an instance field stored inside
    /// a `HeapObject`.
    pub is_static: bool,
}

/// A resolved exception table entry for handler dispatch.
///
/// Built from `duke_classfile::types::ExceptionTableEntry` with `catch_type`
/// resolved from a CP index to a class name string. It acts as a safety net,
/// defining the exact boundaries where a `try` block is active.
///
/// # Examples
///
/// ```
/// use duke_interpreter::context::ExceptionEntry;
///
/// let handler = ExceptionEntry {
///     start_pc: 0,
///     end_pc: 10,
///     handler_pc: 15,
///     catch_type: Some("java/lang/NullPointerException".to_string()),
/// };
/// assert!(handler.catch_type.is_some());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ExceptionEntry {
    /// Inclusive start PC of the try block where this handler becomes active.
    pub start_pc: u16,
    /// Exclusive end PC of the try block where this handler deactivates.
    pub end_pc: u16,
    /// The program counter offset where execution should jump to if the exception matches.
    pub handler_pc: u16,
    /// The specific class of exception to catch (e.g. `"java/io/IOException"`).
    /// If `None`, this handler catches *all* exceptions (typically used for `finally` blocks).
    pub catch_type: Option<String>,
}

/// A parsed class with all methods decoded — the unit of execution for Phase 5+.
///
/// This struct takes the raw data from a `.class` file and fully realizes it into
/// memory, linking the constant pool, method bytecode, and static field states.
///
/// # Examples
///
/// ```
/// use duke_interpreter::context::ClassContext;
///
/// let context = ClassContext {
///     class_name: "java/lang/Object".to_string(),
///     super_class: None,
///     interfaces: vec![],
///     constant_pool: vec![],
///     methods: vec![],
///     fields: vec![],
///     static_fields: vec![],
///     instance_field_count: 0,
///     bootstrap_methods: vec![],
/// };
/// assert_eq!(context.class_name, "java/lang/Object");
/// ```
pub struct ClassContext {
    /// Internal JVM class name (e.g. `"Point"`).
    pub class_name: String,
    /// Superclass name (`None` for `java/lang/Object`).
    pub super_class: Option<String>,
    /// Directly implemented interfaces (used by checkcast / instanceof).
    pub interfaces: Vec<String>,
    /// The fully resolved constant pool, allowing instructions like `ldc` to fetch
    /// strings, classes, and method handles without re-parsing.
    pub constant_pool: Vec<Option<CpEntry>>,
    /// Decoded methods available on this class, ready for execution dispatch.
    pub methods: Vec<MethodEntry>,
    /// All field declarations (static and instance), in class file order.
    pub fields: Vec<FieldEntry>,
    /// Values of static fields, indexed by position among static-only fields.
    pub static_fields: Vec<Slot>,
    /// Number of instance (non-static) fields — used to quickly calculate the size
    /// of new heap objects when a `new` instruction is encountered.
    pub instance_field_count: usize,
    /// `BootstrapMethods` entries from the class attribute (needed for invokedynamic).
    pub bootstrap_methods: Vec<duke_classfile::types::BootstrapMethodEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exception_entry_clone() {
        let entry1 = ExceptionEntry {
            start_pc: 0,
            end_pc: 10,
            handler_pc: 15,
            catch_type: Some("java/lang/Exception".to_string()),
        };

        let entry2 = entry1.clone();
        assert_eq!(entry1.start_pc, entry2.start_pc);
        assert_eq!(entry1.end_pc, entry2.end_pc);
        assert_eq!(entry1.handler_pc, entry2.handler_pc);
        assert_eq!(entry1, entry2);

        let entry3 = ExceptionEntry {
            start_pc: 0,
            end_pc: 10,
            handler_pc: 15,
            catch_type: None,
        };

        let entry4 = entry3.clone();
        assert_eq!(entry3.start_pc, entry4.start_pc);
        assert_eq!(entry3.end_pc, entry4.end_pc);
        assert_eq!(entry3.handler_pc, entry4.handler_pc);
        assert_eq!(entry3, entry4);
    }
}
