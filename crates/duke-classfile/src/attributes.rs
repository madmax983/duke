//! JVM Attributes parsing and representations.
//!
//! This module defines the structures for attributes found in a `.class` file.
//! Attributes are used in the `ClassFile`, `field_info`, `method_info`, and `Code_attribute` structures.
//!
//! While there are many attributes defined in the JVM specification, this module
//! currently parses the following well-known attributes into typed variants:
//! - `Code` (§4.7.3)
//! - `ConstantValue` (§4.7.2)
//! - `SourceFile` (§4.7.10)
//! - `LineNumberTable` (§4.7.12)
//! - `LocalVariableTable` (§4.7.13)
//! - `Exceptions` (§4.7.5)
//! - `BootstrapMethods` (§4.7.23)
//!
//! Unknown attributes are captured as raw bytes in `AttributeData::Raw` for forward compatibility.

use crate::constant_pool::CpIndex;

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
