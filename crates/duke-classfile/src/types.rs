use crate::access_flags::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

/// Newtype wrapper for constant pool indices (1-based per JVM spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CpIndex(pub u16);

/// Parsed top-level class file structure (JVM spec §4.1).
#[derive(Debug, Clone)]
pub struct ClassFile {
    pub minor_version: u16,
    pub major_version: u16,
    /// Constant pool entries. Index 0 is unused (spec is 1-based); Long/Double
    /// entries consume two slots — the phantom slot at N+1 is `None`.
    pub constant_pool: Vec<Option<CpEntry>>,
    pub access_flags: ClassAccessFlags,
    /// Index into constant_pool pointing to CONSTANT_Class for this class.
    pub this_class: CpIndex,
    /// Index into constant_pool pointing to CONSTANT_Class for the superclass,
    /// or 0 for java/lang/Object.
    pub super_class: CpIndex,
    pub interfaces: Vec<CpIndex>,
    pub fields: Vec<FieldInfo>,
    pub methods: Vec<MethodInfo>,
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
    Class { name_index: CpIndex },
    /// Tag 8
    String { string_index: CpIndex },
    /// Tag 9
    Fieldref {
        class_index: CpIndex,
        name_and_type_index: CpIndex,
    },
    /// Tag 10
    Methodref {
        class_index: CpIndex,
        name_and_type_index: CpIndex,
    },
    /// Tag 11
    InterfaceMethodref {
        class_index: CpIndex,
        name_and_type_index: CpIndex,
    },
    /// Tag 12
    NameAndType {
        name_index: CpIndex,
        descriptor_index: CpIndex,
    },
    /// Tag 15 — method handles (JSR 292)
    MethodHandle {
        reference_kind: u8,
        reference_index: CpIndex,
    },
    /// Tag 16
    MethodType { descriptor_index: CpIndex },
    /// Tag 17
    Dynamic {
        bootstrap_method_attr_index: u16,
        name_and_type_index: CpIndex,
    },
    /// Tag 18
    InvokeDynamic {
        bootstrap_method_attr_index: u16,
        name_and_type_index: CpIndex,
    },
    /// Tag 19 — Java 9+ module system
    Module { name_index: CpIndex },
    /// Tag 20 — Java 9+ module system
    Package { name_index: CpIndex },
}

/// Field descriptor (§4.5).
#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub access_flags: FieldAccessFlags,
    pub name_index: CpIndex,
    pub descriptor_index: CpIndex,
    pub attributes: Vec<AttributeInfo>,
}

/// Method descriptor (§4.6).
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub access_flags: MethodAccessFlags,
    pub name_index: CpIndex,
    pub descriptor_index: CpIndex,
    pub attributes: Vec<AttributeInfo>,
}

/// Generic attribute container (§4.7).
/// Well-known attributes are parsed into their typed variants; unknown
/// attributes are captured as raw bytes for forward compatibility.
#[derive(Debug, Clone)]
pub struct AttributeInfo {
    pub name_index: CpIndex,
    pub data: AttributeData,
}

/// Typed attribute payload.
#[derive(Debug, Clone)]
pub enum AttributeData {
    /// Code attribute (§4.7.3) — method bytecode.
    Code(CodeAttribute),
    /// ConstantValue attribute (§4.7.2) — compile-time constant for static fields.
    ConstantValue { constant_value_index: CpIndex },
    /// SourceFile attribute (§4.7.10).
    SourceFile { sourcefile_index: CpIndex },
    /// LineNumberTable (§4.7.12).
    LineNumberTable(Vec<LineNumberEntry>),
    /// LocalVariableTable (§4.7.13).
    LocalVariableTable(Vec<LocalVariableEntry>),
    /// Exceptions attribute (§4.7.5).
    Exceptions { exception_index_table: Vec<CpIndex> },
    /// Any attribute we don't parse in detail yet.
    Raw(Vec<u8>),
}

/// Code attribute payload (§4.7.3).
#[derive(Debug, Clone)]
pub struct CodeAttribute {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Vec<u8>,
    pub exception_table: Vec<ExceptionTableEntry>,
    pub attributes: Vec<AttributeInfo>,
}

/// Exception handler entry within Code attribute.
#[derive(Debug, Clone)]
pub struct ExceptionTableEntry {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    /// 0 means catch-all (finally).
    pub catch_type: CpIndex,
}

/// Single entry in a LineNumberTable attribute.
#[derive(Debug, Clone)]
pub struct LineNumberEntry {
    pub start_pc: u16,
    pub line_number: u16,
}

/// Single entry in a LocalVariableTable attribute.
#[derive(Debug, Clone)]
pub struct LocalVariableEntry {
    pub start_pc: u16,
    pub length: u16,
    pub name_index: CpIndex,
    pub descriptor_index: CpIndex,
    pub index: u16,
}
