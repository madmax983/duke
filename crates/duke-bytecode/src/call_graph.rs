//! Call Graph Generation.
//!
//! This module provides utilities to build a call graph for a given Java class
//! using [Mermaid JS](https://mermaid.js.org/).

use std::collections::BTreeSet;
use std::fmt::Write;

use duke_classfile::{
    ClassFile,
    types::{AttributeData, CpEntry, CpIndex},
};

use crate::decode;

fn cp_str(cf: &ClassFile, idx: CpIndex) -> Option<&str> {
    cf.constant_pool
        .get(idx.0 as usize)
        .and_then(|slot| slot.as_ref())
        .and_then(|entry| {
            if let CpEntry::Utf8(s) = entry {
                Some(s.as_str())
            } else {
                None
            }
        })
}

fn resolve_class_name(cf: &ClassFile, idx: CpIndex) -> &str {
    if idx.0 == 0 {
        return "<none>";
    }
    let class_entry = cf
        .constant_pool
        .get(idx.0 as usize)
        .and_then(|s| s.as_ref());
    if let Some(CpEntry::Class { name_index }) = class_entry {
        cp_str(cf, *name_index).unwrap_or("<invalid utf8>")
    } else {
        "<not a class ref>"
    }
}

fn extract_method_ref(cf: &ClassFile, idx: CpIndex) -> Option<(String, String, String)> {
    let entry = cf.constant_pool.get(idx.0 as usize)?.as_ref()?;

    let (CpEntry::Methodref {
        class_index: class_idx,
        name_and_type_index: nat_idx,
    }
    | CpEntry::InterfaceMethodref {
        class_index: class_idx,
        name_and_type_index: nat_idx,
    }) = entry
    else {
        return None;
    };

    let class_name = resolve_class_name(cf, *class_idx).to_string();

    let nat_entry = cf.constant_pool.get(nat_idx.0 as usize)?.as_ref()?;
    if let CpEntry::NameAndType {
        name_index,
        descriptor_index,
    } = nat_entry
    {
        let method_name = cp_str(cf, *name_index)?.to_string();
        let method_desc = cp_str(cf, *descriptor_index)?.to_string();
        Some((class_name, method_name, method_desc))
    } else {
        None
    }
}

/// Generates a Mermaid call graph (CFG) from a class file.
///
/// **Why it exists:** Navigating method invocations across a codebase manually is
/// error-prone. This function automates the creation of a visual Call Graph by tracing
/// all `invoke*` instructions, making it easier to see dependencies and side effects.
///
/// Generates a Mermaid call graph (CFG) from a given Java class file.
///
/// # Examples
///
/// ```
/// use duke_bytecode::call_graph::generate_mermaid_call_graph;
/// use duke_classfile::{parse, types::ClassFile};
///
/// // Create a dummy ClassFile from valid dummy bytecode:
/// let mut v: Vec<u8> = Vec::new();
/// v.extend_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]); // magic
/// v.extend_from_slice(&[0x00, 0x00, 0x00, 0x41]); // Java 21
/// v.extend_from_slice(&[0x00, 0x03]); // cp_count = 3
/// // #1: Class { name_index = 2 }
/// v.push(7);
/// v.extend_from_slice(&[0x00, 0x02]);
/// // #2: Utf8 "Foo"
/// v.push(1);
/// v.extend_from_slice(&[0x00, 0x03]);
/// v.extend_from_slice(b"Foo");
/// v.extend_from_slice(&[0x00, 0x21, 0x00, 0x01, 0x00, 0x00]); // this=1, super=0
/// v.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // no fields, methods, or attrs
///
/// let cf = parse(&v).unwrap();
/// let graph = generate_mermaid_call_graph(&cf);
/// assert!(graph.contains("graph TD"));
/// // Since we didn't add any methods, the graph will be just "graph TD\n" or similar,
/// // but it shouldn't crash.
/// ```
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
#[must_use]
/// ⚡ Bolt: Using `BTreeSet<String>` removes the need to collect and sort a `Vec` and avoids cloning `source_id` in the hot loop.
pub fn generate_mermaid_call_graph(cf: &ClassFile) -> String {
    let mut cg = String::from("graph TD\n");
    let mut edges = BTreeSet::new();

    let this_class_name = resolve_class_name(cf, cf.this_class);

    for method in &cf.methods {
        let name_str = cp_str(cf, method.name_index).unwrap_or("<invalid>");
        let desc_str = cp_str(cf, method.descriptor_index).unwrap_or("<invalid>");

        let source_id = format!("{this_class_name}::{name_str}{desc_str}");

        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data
                && let Ok(instructions) = decode(&code.code)
            {
                for (_, instr) in instructions {
                    let target_idx = instr.method_invocation_target();

                    if let Some(idx) = target_idx
                        && let Some((target_class, target_method, target_descriptor)) =
                            extract_method_ref(cf, idx)
                    {
                        let target_id =
                            format!("{target_class}::{target_method}{target_descriptor}");
                        edges.insert(format!("    \"{source_id}\" --> \"{target_id}\""));
                    }
                }
            }
        }
    }

    // Output unique edges
    for edge in edges {
        let _ = writeln!(cg, "{edge}");
    }

    cg
}

#[cfg(test)]
mod tests {
    use super::*;
    use duke_classfile::access_flags::{ClassAccessFlags, MethodAccessFlags};
    use duke_classfile::types::{AttributeInfo, CodeAttribute, MethodInfo};

    #[test]
    fn test_generate_mermaid_call_graph_empty() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(0),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };

        let cg = generate_mermaid_call_graph(&cf);
        assert!(cg.contains("graph TD"));
    }

    #[test]
    fn test_resolve_class_name_errors() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None,                       // 0
                Some(CpEntry::Integer(42)), // 1
                Some(CpEntry::Class {
                    name_index: CpIndex(3),
                }), // 2
                Some(CpEntry::Integer(99)), // 3 (not a string)
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(0),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };

        assert_eq!(resolve_class_name(&cf, CpIndex(0)), "<none>");
        assert_eq!(resolve_class_name(&cf, CpIndex(1)), "<not a class ref>");
        assert_eq!(resolve_class_name(&cf, CpIndex(2)), "<invalid utf8>");
    }

    #[test]
    fn test_extract_method_ref_invalid_cases() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None,                       // 0
                Some(CpEntry::Integer(42)), // 1
                Some(CpEntry::Methodref {
                    class_index: CpIndex(0),
                    name_and_type_index: CpIndex(3),
                }), // 2
                Some(CpEntry::Integer(99)), // 3
                Some(CpEntry::Methodref {
                    class_index: CpIndex(0),
                    name_and_type_index: CpIndex(5),
                }), // 4
                Some(CpEntry::NameAndType {
                    name_index: CpIndex(6),
                    descriptor_index: CpIndex(7),
                }), // 5
                Some(CpEntry::Integer(1)),  // 6
                Some(CpEntry::Utf8("()V".to_string())), // 7
                Some(CpEntry::Methodref {
                    class_index: CpIndex(0),
                    name_and_type_index: CpIndex(9),
                }), // 8
                Some(CpEntry::NameAndType {
                    name_index: CpIndex(7),
                    descriptor_index: CpIndex(6),
                }), // 9
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(0),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };

        // Out of bounds
        assert_eq!(extract_method_ref(&cf, CpIndex(99)), None);
        // Not a Methodref
        assert_eq!(extract_method_ref(&cf, CpIndex(1)), None);
        // NameAndType index is not a NameAndType
        assert_eq!(extract_method_ref(&cf, CpIndex(2)), None);
        // NameAndType name index is not a string
        assert_eq!(extract_method_ref(&cf, CpIndex(4)), None);
        // NameAndType desc index is not a string
        assert_eq!(extract_method_ref(&cf, CpIndex(8)), None);
    }

    #[test]
    fn test_generate_mermaid_call_graph_interface_method() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None, // 0
                Some(CpEntry::Class {
                    name_index: CpIndex(2),
                }), // 1
                Some(CpEntry::Utf8("MyInterface".to_string())), // 2
                Some(CpEntry::Utf8("caller".to_string())), // 3
                Some(CpEntry::Utf8("()V".to_string())), // 4
                Some(CpEntry::InterfaceMethodref {
                    class_index: CpIndex(1),
                    name_and_type_index: CpIndex(6),
                }), // 5
                Some(CpEntry::NameAndType {
                    name_index: CpIndex(7),
                    descriptor_index: CpIndex(4),
                }), // 6
                Some(CpEntry::Utf8("target".to_string())), // 7
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(1),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![MethodInfo {
                access_flags: MethodAccessFlags::PUBLIC,
                name_index: CpIndex(3),
                descriptor_index: CpIndex(4),
                attributes: vec![AttributeInfo {
                    name_index: CpIndex(0),
                    data: AttributeData::Code(CodeAttribute {
                        max_stack: 1,
                        max_locals: 1,
                        // invokeinterface #5 1, return
                        code: vec![0xb9, 0x00, 0x05, 0x01, 0x00, 0xb1],
                        exception_table: vec![],
                        attributes: vec![],
                    }),
                }],
            }],
            attributes: vec![],
        };

        let cg = generate_mermaid_call_graph(&cf);
        assert!(cg.contains("\"MyInterface::caller()V\" --> \"MyInterface::target()V\""));
    }

    #[test]
    fn test_generate_mermaid_call_graph_basic() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None, // 0
                Some(CpEntry::Class {
                    name_index: CpIndex(2),
                }), // 1
                Some(CpEntry::Utf8("MyClass".to_string())), // 2
                Some(CpEntry::Utf8("myMethod".to_string())), // 3
                Some(CpEntry::Utf8("()V".to_string())), // 4
                Some(CpEntry::Methodref {
                    class_index: CpIndex(1),
                    name_and_type_index: CpIndex(6),
                }), // 5
                Some(CpEntry::NameAndType {
                    name_index: CpIndex(7),
                    descriptor_index: CpIndex(4),
                }), // 6
                Some(CpEntry::Utf8("targetMethod".to_string())), // 7
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(1),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![MethodInfo {
                access_flags: MethodAccessFlags::PUBLIC,
                name_index: CpIndex(3),
                descriptor_index: CpIndex(4),
                attributes: vec![AttributeInfo {
                    name_index: CpIndex(0),
                    data: AttributeData::Code(CodeAttribute {
                        max_stack: 1,
                        max_locals: 1,
                        // invokestatic #5, return
                        code: vec![0xb8, 0x00, 0x05, 0xb1],
                        exception_table: vec![],
                        attributes: vec![],
                    }),
                }],
            }],
            attributes: vec![],
        };

        let cg = generate_mermaid_call_graph(&cf);
        assert!(cg.contains("\"MyClass::myMethod()V\" --> \"MyClass::targetMethod()V\""));
    }
}
