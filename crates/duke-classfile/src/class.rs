//! Top-level structures for Java class files.
//!
//! This module defines the main `ClassFile` structure, which represents a fully
//! parsed JVM class file. It also provides structures for fields (`FieldInfo`)
//! and methods (`MethodInfo`). These representations closely follow the JVM
//! specification's `ClassFile` layout (§4.1).

use crate::access_flags::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use crate::attributes::AttributeInfo;
use crate::constant_pool::CpIndex;

/// Parsed top-level class file structure (JVM spec §4.1).
///
/// # Examples
///
/// ```
/// use duke_classfile::{ClassFile, CpIndex};
/// use duke_classfile::ClassAccessFlags;
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

/// Represents a field declared within a Java class or interface (§4.5).
///
/// Why do we need this? A class without state is just a namespace of functions.
/// `FieldInfo` defines the layout, type descriptors, and access modifiers of every instance
/// and static field. Crucially, it also carries attributes—such as `ConstantValue` for
/// primitive constants—which tell the JVM how to initialize static variables before any code runs.
///
/// # Examples
///
/// ```
/// use duke_classfile::{FieldInfo, CpIndex};
/// use duke_classfile::FieldAccessFlags;
///
/// let field = FieldInfo {
///     access_flags: FieldAccessFlags::PUBLIC,
///     name_index: CpIndex(1),
///     descriptor_index: CpIndex(2),
///     attributes: vec![],
/// };
/// assert!(field.access_flags.contains(FieldAccessFlags::PUBLIC));
/// ```
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

/// Represents a method or initialization routine within a class (§4.6).
///
/// Methods are the verbs of the JVM. This structure tells you the method's name, its descriptor
/// (what arguments it takes and returns), and its access flags (is it `public`, `static`, or `native`?).
///
/// More importantly, if the method is not `native` or `abstract`, its `attributes` array will contain
/// a [`crate::attributes::AttributeData::Code`] attribute—the raw bytecode instructions the interpreter must execute.
///
/// # Examples
///
/// ```
/// use duke_classfile::{MethodInfo, CpIndex};
/// use duke_classfile::MethodAccessFlags;
///
/// let method = MethodInfo {
///     access_flags: MethodAccessFlags::PUBLIC,
///     name_index: CpIndex(3),
///     descriptor_index: CpIndex(4),
///     attributes: vec![],
/// };
/// assert!(method.access_flags.contains(MethodAccessFlags::PUBLIC));
/// ```
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
