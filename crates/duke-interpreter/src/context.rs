use duke_bytecode::Instruction;
use duke_classfile::types::CpEntry;
use duke_runtime::Slot;

/// A decoded method ready for execution.
pub struct MethodEntry {
    pub name: String,
    pub descriptor: String,
    pub instructions: std::sync::Arc<[(usize, Instruction)]>,
    pub max_stack: u16,
    pub max_locals: u16,
    pub exception_table: Vec<ExceptionEntry>,
    /// Precomputed PC → instruction-index map, shared cheaply via Arc.
    pub pc_to_idx: std::sync::Arc<std::collections::HashMap<usize, usize>>,
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
/// Built from `duke_classfile::types::ExceptionTableEntry` with `catch_type`
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
    /// Directly implemented interfaces (used by checkcast / instanceof).
    pub interfaces: Vec<String>,
    pub constant_pool: Vec<Option<CpEntry>>,
    pub methods: Vec<MethodEntry>,
    /// All field declarations (static and instance), in class file order.
    pub fields: Vec<FieldEntry>,
    /// Values of static fields, indexed by position among static-only fields.
    pub static_fields: Vec<Slot>,
    /// Number of instance (non-static) fields — used to size heap objects at `new`.
    pub instance_field_count: usize,
    /// `BootstrapMethods` entries from the class attribute (needed for invokedynamic).
    pub bootstrap_methods: Vec<duke_classfile::types::BootstrapMethodEntry>,
}
