//! Call Graph Generation.
//!
//! This module provides utilities to build a call graph for a given Java class
//! using [Mermaid JS](https://mermaid.js.org/).

use std::collections::HashSet;
use std::fmt::Write;

use duke_classfile::{
    ClassFile,
    types::{AttributeData, CpEntry, CpIndex},
};

use crate::{Instruction, decode};

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

    let (class_idx, nat_idx) = match entry {
        CpEntry::Methodref { class_index, name_and_type_index } => (class_index, name_and_type_index),
        CpEntry::InterfaceMethodref { class_index, name_and_type_index } => (class_index, name_and_type_index),
        _ => return None,
    };

    let class_name = resolve_class_name(cf, *class_idx).to_string();

    let nat_entry = cf.constant_pool.get(nat_idx.0 as usize)?.as_ref()?;
    if let CpEntry::NameAndType { name_index, descriptor_index } = nat_entry {
        let name = cp_str(cf, *name_index)?.to_string();
        let desc = cp_str(cf, *descriptor_index)?.to_string();
        Some((class_name, name, desc))
    } else {
        None
    }
}


/// Generates a Mermaid call graph (CFG) from a class file.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
#[must_use]
pub fn generate_mermaid_call_graph(cf: &ClassFile) -> String {
    let mut cg = String::from("graph TD\n");
    let mut edges = HashSet::new();

    let this_class_name = resolve_class_name(cf, cf.this_class);

    for method in &cf.methods {
        let name_str = cp_str(cf, method.name_index).unwrap_or("<invalid>");
        let desc_str = cp_str(cf, method.descriptor_index).unwrap_or("<invalid>");

        let caller_id = format!("{this_class_name}::{name_str}{desc_str}");

        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data {
                if let Ok(instructions) = decode(&code.code) {
                    for (_, instr) in instructions {
                        let target_idx = match instr {
                            Instruction::Invokevirtual(idx) |
                            Instruction::Invokespecial(idx) |
                            Instruction::Invokestatic(idx) |
                            Instruction::Invokeinterface { index: idx, .. } => {
                                Some(idx)
                            }
                            // Invokedynamic is more complex, skip for basic call graph
                            _ => None,
                        };

                        if let Some(idx) = target_idx {
                            if let Some((target_class, target_name, target_desc)) = extract_method_ref(cf, idx) {
                                let callee_id = format!("{target_class}::{target_name}{target_desc}");
                                edges.insert((caller_id.clone(), callee_id));
                            }
                        }
                    }
                }
            }
        }
    }

    // Output unique edges
    let mut sorted_edges: Vec<_> = edges.into_iter().collect();
    sorted_edges.sort(); // For deterministic output

    for (caller, callee) in sorted_edges {
        let _ = writeln!(cg, "    \"{}\" --> \"{}\"", caller, callee);
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
    fn test_generate_mermaid_call_graph_basic() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None, // 0
                Some(CpEntry::Class { name_index: CpIndex(2) }), // 1
                Some(CpEntry::Utf8("MyClass".to_string())), // 2
                Some(CpEntry::Utf8("myMethod".to_string())), // 3
                Some(CpEntry::Utf8("()V".to_string())), // 4
                Some(CpEntry::Methodref { class_index: CpIndex(1), name_and_type_index: CpIndex(6) }), // 5
                Some(CpEntry::NameAndType { name_index: CpIndex(7), descriptor_index: CpIndex(4) }), // 6
                Some(CpEntry::Utf8("targetMethod".to_string())), // 7
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(1),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![
                MethodInfo {
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
                },
            ],
            attributes: vec![],
        };

        let cg = generate_mermaid_call_graph(&cf);
        assert!(cg.contains("\"MyClass::myMethod()V\" --> \"MyClass::targetMethod()V\""));
    }
}
