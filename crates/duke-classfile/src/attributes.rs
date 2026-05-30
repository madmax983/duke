//! JVM Attributes parsing and representations.
//!
//! In the JVM, attributes are the flexible extensibility mechanism of a `.class` file.
//! They contain the *actual* bytecode of a method, the source file name, debugging symbols,
//! or bootstrap methods for `invokedynamic`. If a class file were a book, attributes would
//! be the footnotes, the appendices, and occasionally the entire plot!
//!
//! This module parses known attributes into typed variants (like [`AttributeData::Code`]),
//! ensuring you can inspect the method's behavior. Unknown attributes are preserved
//! as [`AttributeData::Raw`] so the runtime won't crash on newer compiler features it
//! doesn't fully understand yet.
//!
//! We support:
//! - `Code` (§4.7.3)
//! - `ConstantValue` (§4.7.2)
//! - `SourceFile` (§4.7.10)
//! - `LineNumberTable` (§4.7.12)
//! - `LocalVariableTable` (§4.7.13)
//! - `Exceptions` (§4.7.5)
//! - `BootstrapMethods` (§4.7.23)

use crate::constant_pool::CpIndex;

/// Generic attribute container (§4.7).
///
/// Every attribute starts with a name (a UTF-8 string in the constant pool) and a length.
/// We parse this wrapper, and then decode the inner payload into an [`AttributeData`] enum.
/// If we don't recognize the attribute, it lives on safely as a raw byte vector, allowing
/// the interpreter to ignore unknown metadata rather than failing to load the class.
///
/// # Examples
///
/// ```
/// use duke_classfile::{AttributeInfo, AttributeData, CpIndex};
///
/// let attr = AttributeInfo {
///     name_index: CpIndex(1),
///     data: AttributeData::Raw(vec![0x01, 0x02]),
/// };
///
/// if let AttributeData::Raw(bytes) = attr.data {
///     assert_eq!(bytes.len(), 2);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct AttributeInfo {
    /// Constant pool index of the name of the attribute.
    pub name_index: CpIndex,
    /// The parsed attribute data.
    pub data: AttributeData,
}

/// Single entry in the `BootstrapMethods` attribute (§4.7.23).
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::{BootstrapMethodEntry, CpIndex};
/// let b = BootstrapMethodEntry { bootstrap_method_ref: CpIndex(1), bootstrap_arguments: vec![] };
/// ```
/// # Examples
/// ```
/// use duke_classfile::{BootstrapMethodEntry, CpIndex};
/// let b = BootstrapMethodEntry { bootstrap_method_ref: CpIndex(1), bootstrap_arguments: vec![] };
/// ```
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::{BootstrapMethodEntry, CpIndex};
/// let b = BootstrapMethodEntry { bootstrap_method_ref: CpIndex(1), bootstrap_arguments: vec![] };
/// ```
pub struct BootstrapMethodEntry {
    /// CP index pointing to a `CONSTANT_MethodHandle`.
    pub method_ref: CpIndex,
    /// CP indices pointing to static arguments (String, `MethodType`, `MethodHandle`, etc.).
    pub arguments: Vec<CpIndex>,
}

/// One parsed runtime annotation instance (§4.7.16.1).
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::{Annotation, CpIndex};
/// let a = Annotation { type_index: CpIndex(1), element_value_pairs: vec![] };
/// ```
/// # Examples
/// ```
/// use duke_classfile::{Annotation, CpIndex};
/// let a = Annotation { type_index: CpIndex(1), element_value_pairs: vec![] };
/// ```
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::{Annotation, CpIndex};
/// let a = Annotation { type_index: CpIndex(1), element_value_pairs: vec![] };
/// ```
pub struct Annotation {
    /// Type descriptor of the annotation (`type_index` in the class file).
    pub type_index: CpIndex,
    /// Named element value pairs declared on the annotation usage.
    pub element_value_pairs: Vec<ElementValuePair>,
}

/// One `name=value` pair in an annotation usage (§4.7.16.1).
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::{ElementValuePair, CpIndex, ElementValue};
/// let p = ElementValuePair { element_name_index: CpIndex(1), value: ElementValue::ClassInfoIndex(CpIndex(2)) };
/// ```
/// # Examples
/// ```
/// use duke_classfile::{ElementValuePair, CpIndex, ElementValue};
/// let p = ElementValuePair { element_name_index: CpIndex(1), value: ElementValue::ClassInfoIndex(CpIndex(2)) };
/// ```
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::{ElementValuePair, CpIndex, ElementValue};
/// let p = ElementValuePair { element_name_index: CpIndex(1), value: ElementValue::ClassInfoIndex(CpIndex(2)) };
/// ```
pub struct ElementValuePair {
    /// Constant pool index of the element (method) name in the annotation interface.
    pub element_name_index: CpIndex,
    /// Encoded value payload.
    pub value: ElementValue,
}

/// Encoded annotation element value (§4.7.16.1).
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::{ElementValue, CpIndex};
/// let e = ElementValue::ClassInfoIndex(CpIndex(1));
/// ```
/// # Examples
/// ```
/// use duke_classfile::{ElementValue, CpIndex};
/// let e = ElementValue::ClassInfoIndex(CpIndex(1));
/// ```
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::{ElementValue, CpIndex};
/// let e = ElementValue::ClassInfoIndex(CpIndex(1));
/// ```
pub enum ElementValue {
    /// Primitive/String constant pool reference.
    ConstValueIndex(CpIndex),
    /// Enum element value.
    EnumConstValue {
        /// Descriptor of enum type.
        type_name_index: CpIndex,
        /// Enum constant simple name.
        const_name_index: CpIndex,
    },
    /// Class literal element (`Class<?>`).
    ClassInfoIndex(CpIndex),
    /// Nested annotation element.
    AnnotationValue(Annotation),
    /// Array element containing child element values.
    ArrayValue(Vec<Self>),
}

/// Typed attribute payload.
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::{AttributeData, CpIndex};
/// let a = AttributeData::ConstantValue { constant_value_index: CpIndex(1) };
/// ```
/// # Examples
/// ```
/// use duke_classfile::{AttributeData, CpIndex};
/// let a = AttributeData::ConstantValue { constant_value_index: CpIndex(1) };
/// ```
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::{AttributeData, CpIndex};
/// let a = AttributeData::ConstantValue { constant_value_index: CpIndex(1) };
/// ```
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
    /// `RuntimeVisibleAnnotations` (§4.7.16).
    RuntimeVisibleAnnotations(Vec<Annotation>),
    /// `AnnotationDefault` (§4.7.22) default value for an annotation element.
    AnnotationDefault(ElementValue),
    /// Any attribute we don't parse in detail yet.
    Raw(Vec<u8>),
}

/// Code attribute payload (§4.7.3).
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::CodeAttribute;
/// let c = CodeAttribute { max_stack: 1, max_locals: 1, code: vec![], exception_table: vec![], attributes: vec![] };
/// ```
/// # Examples
/// ```
/// use duke_classfile::CodeAttribute;
/// let c = CodeAttribute { max_stack: 1, max_locals: 1, code: vec![], exception_table: vec![], attributes: vec![] };
/// ```
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::CodeAttribute;
/// let c = CodeAttribute { max_stack: 1, max_locals: 1, code: vec![], exception_table: vec![], attributes: vec![] };
/// ```
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
/// # Examples
/// ```
/// use duke_classfile::{ExceptionTableEntry, CpIndex};
/// let e = ExceptionTableEntry { start_pc: 0, end_pc: 0, handler_pc: 0, catch_type: None };
/// ```
/// # Examples
/// ```
/// use duke_classfile::{ExceptionTableEntry, CpIndex};
/// let e = ExceptionTableEntry { start_pc: 0, end_pc: 0, handler_pc: 0, catch_type: None };
/// ```
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::{ExceptionTableEntry, CpIndex};
/// let e = ExceptionTableEntry { start_pc: 0, end_pc: 0, handler_pc: 0, catch_type: None };
/// ```
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
/// # Examples
/// ```
/// use duke_classfile::LineNumberEntry;
/// let l = LineNumberEntry { start_pc: 0, line_number: 1 };
/// ```
/// # Examples
/// ```
/// use duke_classfile::LineNumberEntry;
/// let l = LineNumberEntry { start_pc: 0, line_number: 1 };
/// ```
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::LineNumberEntry;
/// let l = LineNumberEntry { start_pc: 0, line_number: 1 };
/// ```
pub struct LineNumberEntry {
    /// The index into the `code` array at which the code for a new line in the original source file begins.
    pub start_pc: u16,
    /// The corresponding line number in the original source file.
    pub line_number: u16,
}

/// Single entry in a `LocalVariableTable` attribute.
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::{LocalVariableEntry, CpIndex};
/// let l = LocalVariableEntry { start_pc: 0, length: 0, name_index: CpIndex(1), descriptor_index: CpIndex(2), index: 0 };
/// ```
/// # Examples
/// ```
/// use duke_classfile::{LocalVariableEntry, CpIndex};
/// let l = LocalVariableEntry { start_pc: 0, length: 0, name_index: CpIndex(1), descriptor_index: CpIndex(2), index: 0 };
/// ```
#[derive(Debug, Clone)]
/// # Examples
/// ```
/// use duke_classfile::{LocalVariableEntry, CpIndex};
/// let l = LocalVariableEntry { start_pc: 0, length: 0, name_index: CpIndex(1), descriptor_index: CpIndex(2), index: 0 };
/// ```
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
