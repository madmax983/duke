use crate::access_flags::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

/// Newtype wrapper for constant pool indices (1-based per JVM spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CpIndex(pub u16);

/// Parsed top-level class file structure (JVM spec §4.1).
#[derive(Debug, Clone)]
pub struct ClassFile {
    /// Minor version of the class file format.
    pub minor_version: u16,
    /// Major version of the class file format.
    pub major_version: u16,
    /// Constant pool entries. Index 0 is unused (spec is 1-based); Long/Double
    /// entries consume two slots — the phantom slot at N+1 is `None`.
    pub constant_pool: Vec<Option<CpEntry>>,
    /// Access permissions and properties of this class or interface.
    pub access_flags: ClassAccessFlags,
    /// Index into `constant_pool` pointing to `CONSTANT_Class` for this class.
    pub this_class: CpIndex,
    /// Index into `constant_pool` pointing to `CONSTANT_Class` for the superclass,
    /// or 0 for java/lang/Object.
    pub super_class: CpIndex,
    /// Array of interfaces that this class implements.
    pub interfaces: Vec<CpIndex>,
    /// Array of fields declared by this class or interface.
    pub fields: Vec<FieldInfo>,
    /// Array of methods declared by this class or interface.
    pub methods: Vec<MethodInfo>,
    /// Array of attributes associated with this class.
    pub attributes: Vec<AttributeInfo>,
}

/// All constant pool entry kinds defined in JVM SE 21 (§4.4).
#[derive(Debug, Clone, PartialEq)]
pub enum CpEntry {
    /// Tag 1
    Utf8(String),
    /// Tag 3
    Integer(i32),
    /// Tag 4
    Float(f32),
    /// Tag 5 — occupies two slots; next slot will be `None`
    Long(i64),
    /// Tag 6 — occupies two slots; next slot will be `None`
    Double(f64),
    /// Tag 7
    Class {
        /// Index to a `CONSTANT_Utf8` structure representing a valid binary class or interface name.
        name_index: CpIndex,
    },
    /// Tag 8
    String {
        /// Index to a `CONSTANT_Utf8` structure representing the string's value.
        string_index: CpIndex,
    },
    /// Tag 9
    Fieldref {
        /// Index to a `CONSTANT_Class` structure.
        class_index: CpIndex,
        /// Index to a `CONSTANT_NameAndType` structure.
        name_and_type_index: CpIndex,
    },
    /// Tag 10
    Methodref {
        /// Index to a `CONSTANT_Class` structure.
        class_index: CpIndex,
        /// Index to a `CONSTANT_NameAndType` structure.
        name_and_type_index: CpIndex,
    },
    /// Tag 11
    InterfaceMethodref {
        /// Index to a `CONSTANT_Class` structure.
        class_index: CpIndex,
        /// Index to a `CONSTANT_NameAndType` structure.
        name_and_type_index: CpIndex,
    },
    /// Tag 12
    NameAndType {
        /// Index to a `CONSTANT_Utf8` structure representing a valid unqualified name.
        name_index: CpIndex,
        /// Index to a `CONSTANT_Utf8` structure representing a valid field or method descriptor.
        descriptor_index: CpIndex,
    },
    /// Tag 15 — method handles (JSR 292)
    MethodHandle {
        /// The kind of method handle.
        reference_kind: u8,
        /// The reference index for the method handle.
        reference_index: CpIndex,
    },
    /// Tag 16
    MethodType {
        /// Index to a `CONSTANT_Utf8` structure representing a method descriptor.
        descriptor_index: CpIndex,
    },
    /// Tag 17
    Dynamic {
        /// An index into the bootstrap method table.
        bootstrap_method_attr_index: u16,
        /// Index to a `CONSTANT_NameAndType` structure.
        name_and_type_index: CpIndex,
    },
    /// Tag 18
    InvokeDynamic {
        /// An index into the bootstrap method table.
        bootstrap_method_attr_index: u16,
        /// Index to a `CONSTANT_NameAndType` structure.
        name_and_type_index: CpIndex,
    },
    /// Tag 19 — Java 9+ module system
    Module {
        /// Index to a `CONSTANT_Utf8` structure representing a module name.
        name_index: CpIndex,
    },
    /// Tag 20 — Java 9+ module system
    Package {
        /// Index to a `CONSTANT_Utf8` structure representing a package name.
        name_index: CpIndex,
    },
}

/// Field descriptor (§4.5).
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// Access flags for the field.
    pub access_flags: FieldAccessFlags,
    /// Constant pool index of the name of the field.
    pub name_index: CpIndex,
    /// Constant pool index of the descriptor of the field.
    pub descriptor_index: CpIndex,
    /// Attributes associated with the field.
    pub attributes: Vec<AttributeInfo>,
}

/// Method descriptor (§4.6).
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// Access flags for the method.
    pub access_flags: MethodAccessFlags,
    /// Constant pool index of the name of the method.
    pub name_index: CpIndex,
    /// Constant pool index of the descriptor of the method.
    pub descriptor_index: CpIndex,
    /// Attributes associated with the method.
    pub attributes: Vec<AttributeInfo>,
}

/// Generic attribute container (§4.7).
/// Well-known attributes are parsed into their typed variants; unknown
/// attributes are captured as raw bytes for forward compatibility.
#[derive(Debug, Clone)]
pub struct AttributeInfo {
    /// Constant pool index of the name of the attribute.
    pub name_index: CpIndex,
    /// The parsed attribute data.
    pub data: AttributeData,
}

/// Single entry in the `BootstrapMethods` attribute (§4.7.23).
#[derive(Debug, Clone)]
pub struct BootstrapMethodEntry {
    /// CP index pointing to a `CONSTANT_MethodHandle`.
    pub method_ref: CpIndex,
    /// CP indices pointing to static arguments (String, `MethodType`, `MethodHandle`, etc.).
    pub arguments: Vec<CpIndex>,
}

/// Typed attribute payload.
#[derive(Debug, Clone)]
pub enum AttributeData {
    /// Code attribute (§4.7.3) — method bytecode.
    Code(CodeAttribute),
    /// `ConstantValue` attribute (§4.7.2) — compile-time constant for static fields.
    ConstantValue {
        /// Index into the constant pool.
        constant_value_index: CpIndex,
    },
    /// `SourceFile` attribute (§4.7.10).
    SourceFile {
        /// Index into the constant pool representing the name of the source file.
        sourcefile_index: CpIndex,
    },
    /// `LineNumberTable` (§4.7.12).
    LineNumberTable(Vec<LineNumberEntry>),
    /// `LocalVariableTable` (§4.7.13).
    LocalVariableTable(Vec<LocalVariableEntry>),
    /// Exceptions attribute (§4.7.5).
    Exceptions {
        /// Table of exception indices.
        exception_index_table: Vec<CpIndex>,
    },
    /// `BootstrapMethods` attribute (§4.7.23) — required for invokedynamic.
    BootstrapMethods(Vec<BootstrapMethodEntry>),
    /// Any attribute we don't parse in detail yet.
    Raw(Vec<u8>),
}

/// Code attribute payload (§4.7.3).
#[derive(Debug, Clone)]
pub struct CodeAttribute {
    /// Maximum depth of the operand stack of this method at any point during execution.
    pub max_stack: u16,
    /// Number of local variables in the local variable array allocated upon invocation of this method.
    pub max_locals: u16,
    /// The actual bytes of JVM instructions that implement the method.
    pub code: Vec<u8>,
    /// Table of exception handlers for this method.
    pub exception_table: Vec<ExceptionTableEntry>,
    /// Additional attributes associated with this code (e.g., `LineNumberTable`, `LocalVariableTable`).
    pub attributes: Vec<AttributeInfo>,
}

/// Exception handler entry within Code attribute.
#[derive(Debug, Clone)]
pub struct ExceptionTableEntry {
    /// The start of the range in the `code` array at which the exception handler is active.
    pub start_pc: u16,
    /// The end of the range in the `code` array at which the exception handler is active.
    pub end_pc: u16,
    /// The start of the exception handler code within the `code` array.
    pub handler_pc: u16,
    /// 0 means catch-all (finally).
    pub catch_type: CpIndex,
}

/// Single entry in a `LineNumberTable` attribute.
#[derive(Debug, Clone)]
pub struct LineNumberEntry {
    /// The index into the `code` array at which the code for a new line in the original source file begins.
    pub start_pc: u16,
    /// The corresponding line number in the original source file.
    pub line_number: u16,
}

/// Single entry in a `LocalVariableTable` attribute.
#[derive(Debug, Clone)]
pub struct LocalVariableEntry {
    /// The index into the `code` array at which the local variable must have a value.
    pub start_pc: u16,
    /// The length of the range in the `code` array for which the local variable has a value.
    pub length: u16,
    /// The constant pool index of the name of this local variable.
    pub name_index: CpIndex,
    /// The constant pool index of the field descriptor for this local variable.
    pub descriptor_index: CpIndex,
    /// The index into the local variable array of the current frame where this local variable is stored.
    pub index: u16,
}
