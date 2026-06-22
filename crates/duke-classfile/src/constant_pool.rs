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
#[derive(Debug, Clone, PartialEq)]
pub enum CpEntry {
    /// A `CONSTANT_Utf8` structure (Tag 1).
    ///
    /// Contains a string of valid JVM-modified UTF-8 text. Used for names of classes, methods, and fields,
    /// as well as string literals in the bytecode.
    ///
    /// # Details
    ///
    /// Modified UTF-8 uses a slightly different encoding than standard UTF-8 (e.g. representing the null character
    /// `\0` as a two-byte sequence). The parser automatically converts these sequences into standard Rust `String`s.
    Utf8(String),
    /// A `CONSTANT_Integer` structure (Tag 3).
    ///
    /// Represents a 32-bit integer constant.
    Integer(i32),
    /// A `CONSTANT_Float` structure (Tag 4).
    ///
    /// Represents a 32-bit floating-point constant.
    Float(f32),
    /// A `CONSTANT_Long` structure (Tag 5).
    ///
    /// Represents a 64-bit integer constant.
    ///
    /// # Usage
    ///
    /// In the JVM constant pool, a `CONSTANT_Long` occupies **two** slots.
    /// The parsing phase accounts for this by leaving the next index in the pool array empty (`None`).
    Long(i64),
    /// A `CONSTANT_Double` structure (Tag 6).
    ///
    /// Represents a 64-bit floating-point constant.
    ///
    /// # Usage
    ///
    /// Like `CONSTANT_Long`, this occupies **two** slots in the constant pool array.
    Double(f64),
    /// A `CONSTANT_Class` structure (Tag 7).
    ///
    /// Represents a class or interface type.
    Class {
        /// Index to a `CONSTANT_Utf8` structure representing a valid binary class or interface name.
        name_index: CpIndex,
    },
    /// A `CONSTANT_String` structure (Tag 8).
    ///
    /// Represents a constant string object.
    String {
        /// Index to a `CONSTANT_Utf8` structure representing the string's value.
        string_index: CpIndex,
    },
    /// A `CONSTANT_Fieldref` structure (Tag 9).
    ///
    /// Represents a reference to a field.
    Fieldref {
        /// Index to a `CONSTANT_Class` structure.
        class_index: CpIndex,
        /// Index to a `CONSTANT_NameAndType` structure.
        name_and_type_index: CpIndex,
    },
    /// A `CONSTANT_Methodref` structure (Tag 10).
    ///
    /// Represents a reference to a class's method.
    Methodref {
        /// Index to a `CONSTANT_Class` structure.
        class_index: CpIndex,
        /// Index to a `CONSTANT_NameAndType` structure.
        name_and_type_index: CpIndex,
    },
    /// A `CONSTANT_InterfaceMethodref` structure (Tag 11).
    ///
    /// Represents a reference to an interface's method.
    InterfaceMethodref {
        /// Index to a `CONSTANT_Class` structure.
        class_index: CpIndex,
        /// Index to a `CONSTANT_NameAndType` structure.
        name_and_type_index: CpIndex,
    },
    /// A `CONSTANT_NameAndType` structure (Tag 12).
    ///
    /// Represents a field or method, without indicating which class or interface type it belongs to.
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
    /// A `CONSTANT_MethodType` structure (Tag 16).
    ///
    /// Represents a method type.
    MethodType {
        /// Index to a `CONSTANT_Utf8` structure representing a method descriptor.
        descriptor_index: CpIndex,
    },
    /// A `CONSTANT_Dynamic` structure (Tag 17).
    ///
    /// Used by an `invokedynamic` instruction to specify a dynamically-computed constant.
    Dynamic {
        /// An index into the bootstrap method table.
        bootstrap_method_attr_index: u16,
        /// Index to a `CONSTANT_NameAndType` structure.
        name_and_type_index: CpIndex,
    },
    /// A `CONSTANT_InvokeDynamic` structure (Tag 18).
    ///
    /// Used by an `invokedynamic` instruction to specify a dynamically-computed call site.
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
