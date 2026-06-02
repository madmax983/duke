//! Constant pool entries and indices.
//!
//! This module defines structures that represent entries in the Java class file's constant pool.
//! The constant pool is a table of structures representing various string constants, class and interface names,
//! field names, and other constants that are referred to within the `ClassFile` structure and its substructures.
//!
//! The `CpEntry` enum covers all constant pool entry tags described in JVM SE 21 (§4.4), including Java 9+
//! modules and packages.

/// Newtype wrapper for constant pool indices (1-based per JVM spec).
///
/// # Examples
///
/// ```
/// use duke_classfile::CpIndex;
///
/// let index = CpIndex(42);
/// assert_eq!(index.0, 42);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CpIndex(pub u16);

/// All constant pool entry kinds defined in JVM SE 21 (§4.4).
///
/// This enum represents the different types of constants that can appear in the constant pool.
///
/// # Examples
///
/// ```
/// use duke_classfile::{CpEntry, CpIndex};
///
/// let utf8_entry = CpEntry::Utf8("java/lang/Object".to_string());
/// let class_entry = CpEntry::Class { name_index: CpIndex(1) };
///
/// assert!(matches!(class_entry, CpEntry::Class { .. }));
/// ```
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
