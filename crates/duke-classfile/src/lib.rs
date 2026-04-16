//! `duke-classfile` — JVM `.class` file parser.
//!
//! Parses `.class` files conforming to JVM SE 21 (class file version 65).
//! All constant pool entries for JDK 21 are supported, including the Java 9+
//! Module and Package entry kinds.
//!
//! # Example
//!
//! ```no_run
//! use duke_classfile::parse;
//!
//! let bytes = std::fs::read("HelloWorld.class").unwrap();
//! let class_file = parse(&bytes).expect("valid .class file");
//! println!("Class: {:?}", class_file.this_class);
//! println!("Methods: {}", class_file.methods.len());
//! ```

pub mod access_flags;
pub mod attributes;
pub mod class;
pub mod constant_pool;
pub mod error;
pub mod parser;

/// Compatibility module re-exporting the split type structures.
pub mod types {
    pub use crate::attributes::*;
    pub use crate::class::*;
    pub use crate::constant_pool::*;
}

pub use error::{Error, ParseError, ParseResult, Result};
pub use parser::parse;
pub use types::{
    AttributeData, AttributeInfo, ClassFile, CodeAttribute, CpEntry, CpIndex, ExceptionTableEntry,
    FieldInfo, LineNumberEntry, LocalVariableEntry, MethodInfo,
};

#[cfg(test)]
mod fuzz;
#[cfg(test)]
mod tests;
