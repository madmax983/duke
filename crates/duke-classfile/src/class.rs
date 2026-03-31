use crate::access_flags::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use crate::attributes::AttributeInfo;
use crate::constant_pool::CpIndex;

/// Parsed top-level class file structure (JVM spec §4.1).
///
/// # Examples
///
/// ```
/// use duke_classfile::types::{ClassFile, CpIndex};
/// use duke_classfile::access_flags::ClassAccessFlags;
///
/// let class_file = ClassFile {
///     minor_version: 0,
///     major_version: 65, // Java 21
///     constant_pool: vec![None], // 1-based index, index 0 is unused
///     access_flags: ClassAccessFlags::PUBLIC,
///     this_class: CpIndex(1),
///     super_class: CpIndex(0), // java/lang/Object
///     interfaces: vec![],
///     fields: vec![],
///     methods: vec![],
///     attributes: vec![],
/// };
///
/// assert_eq!(class_file.major_version, 65);
/// ```
#[derive(Debug, Clone)]
pub struct ClassFile {
    /// Minor version of the class file format.
    pub minor_version: u16,
    /// Major version of the class file format.
    pub major_version: u16,
    /// Constant pool entries. Index 0 is unused (spec is 1-based); Long/Double
    /// entries consume two slots — the phantom slot at N+1 is `None`.
    pub constant_pool: Vec<Option<crate::constant_pool::CpEntry>>,
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
