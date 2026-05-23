//! UML Class Diagram Generation.
//!
//! This module provides utilities to convert a parsed Java class file into a visual
//! representation using [Mermaid JS](https://mermaid.js.org/) `classDiagram`.

use duke_classfile::{
    ClassFile, FieldAccessFlags, MethodAccessFlags, {CpEntry, CpIndex},
};
use std::fmt::Write;

fn cp_str(cf: &ClassFile, idx: CpIndex) -> Option<&str> {
    cf.constant_pool
        .get(idx.0 as usize)
        .and_then(|slot: &Option<duke_classfile::CpEntry>| slot.as_ref())
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
        .and_then(|s: &Option<duke_classfile::CpEntry>| s.as_ref());
    if let Some(CpEntry::Class { name_index }) = class_entry {
        cp_str(cf, *name_index).unwrap_or("<invalid utf8>")
    } else {
        "<not a class ref>"
    }
}

const fn field_visibility(flags: FieldAccessFlags) -> &'static str {
    if flags.contains(FieldAccessFlags::PUBLIC) {
        "+"
    } else if flags.contains(FieldAccessFlags::PRIVATE) {
        "-"
    } else if flags.contains(FieldAccessFlags::PROTECTED) {
        "#"
    } else {
        "~"
    }
}

const fn method_visibility(flags: MethodAccessFlags) -> &'static str {
    if flags.contains(MethodAccessFlags::PUBLIC) {
        "+"
    } else if flags.contains(MethodAccessFlags::PRIVATE) {
        "-"
    } else if flags.contains(MethodAccessFlags::PROTECTED) {
        "#"
    } else {
        "~"
    }
}

/// Generates a Mermaid class diagram from a `ClassFile`.
///
/// **Why it exists:** Looking at raw class structures or decompiled bytecode is tedious.
/// This function translates the structural metadata of a class file into a visual
/// diagram, bridging the gap between machine code and human-readable architecture models.
///
/// This provides a visual representation of the class's fields, methods,
/// and its inheritance hierarchy (superclass and implemented interfaces) using
/// Mermaid JS `classDiagram` syntax.
///
/// # Examples
///
/// ```ignore
/// // Assumes a `ClassFile` structure exists.
/// use duke::uml::generate_mermaid_uml;
/// useClassFile;
///
/// let cf: ClassFile = get_parsed_class_somehow();
/// let diagram_string = generate_mermaid_uml(&cf);
/// println!("{}", diagram_string);
/// ```
#[must_use]
pub fn generate_mermaid_uml(cf: &ClassFile) -> String {
    let mut out = String::from("classDiagram\n");

    let raw_class_name = resolve_class_name(cf, cf.this_class);
    let class_name = raw_class_name.replace('/', "_");

    let _ = writeln!(&mut out, "    class {class_name} {{");

    for field in &cf.fields {
        let vis = field_visibility(field.access_flags);
        let name = cp_str(cf, field.name_index).unwrap_or("<invalid>");
        let desc = cp_str(cf, field.descriptor_index).unwrap_or("<invalid>");

        let _ = writeln!(&mut out, "        {vis}{desc} {name}");
    }

    for method in &cf.methods {
        let vis = method_visibility(method.access_flags);
        let name = cp_str(cf, method.name_index).unwrap_or("<invalid>");
        let desc = cp_str(cf, method.descriptor_index).unwrap_or("<invalid>");

        let _ = writeln!(&mut out, "        {vis}{name}{desc}");
    }

    out.push_str("    }\n");

    if cf.super_class.0 != 0 {
        let raw_super_name = resolve_class_name(cf, cf.super_class);
        if raw_super_name != "java/lang/Object" {
            let super_name = raw_super_name.replace('/', "_");
            let _ = writeln!(&mut out, "    {super_name} <|-- {class_name}");
        }
    }

    for &interface_idx in &cf.interfaces {
        let raw_iface_name = resolve_class_name(cf, interface_idx);
        let iface_name = raw_iface_name.replace('/', "_");
        let _ = writeln!(&mut out, "    {iface_name} <|.. {class_name}");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use duke_classfile::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
    use duke_classfile::{FieldInfo, MethodInfo};

    #[test]
    fn test_generate_mermaid_uml() {
        let cf = ClassFile {
            minor_version: 0,
            major_version: 52,
            constant_pool: vec![
                None,
                Some(CpEntry::Utf8("java/lang/Object".to_string())),
                Some(CpEntry::Class {
                    name_index: CpIndex(1),
                }),
                Some(CpEntry::Utf8("MyClass".to_string())),
                Some(CpEntry::Class {
                    name_index: CpIndex(3),
                }),
                Some(CpEntry::Utf8("myField".to_string())),
                Some(CpEntry::Utf8("I".to_string())),
                Some(CpEntry::Utf8("myMethod".to_string())),
                Some(CpEntry::Utf8("()V".to_string())),
                Some(CpEntry::Utf8("Runnable".to_string())),
                Some(CpEntry::Class {
                    name_index: CpIndex(9),
                }),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(4),
            super_class: CpIndex(2),
            interfaces: vec![CpIndex(10)],
            fields: vec![FieldInfo {
                access_flags: FieldAccessFlags::PRIVATE,
                name_index: CpIndex(5),
                descriptor_index: CpIndex(6),
                attributes: vec![],
            }],
            methods: vec![MethodInfo {
                access_flags: MethodAccessFlags::PUBLIC,
                name_index: CpIndex(7),
                descriptor_index: CpIndex(8),
                attributes: vec![],
            }],
            attributes: vec![],
        };

        let uml = generate_mermaid_uml(&cf);
        assert!(uml.contains("classDiagram"));
        assert!(uml.contains("class MyClass {"));
        assert!(uml.contains("-I myField"));
        assert!(uml.contains("+myMethod()V"));
        assert!(uml.contains("Runnable <|.. MyClass"));
    }

    #[test]
    fn test_generate_mermaid_uml_visibilities_and_invalid() {
        let cf = ClassFile {
            minor_version: 0,
            major_version: 52,
            constant_pool: vec![
                None,
                Some(CpEntry::Utf8("ValidClass".to_string())),
                Some(CpEntry::Class {
                    name_index: CpIndex(1),
                }),
                Some(CpEntry::Utf8("SuperClass".to_string())),
                Some(CpEntry::Class {
                    name_index: CpIndex(3),
                }),
                Some(CpEntry::Utf8("protectedField".to_string())),
                Some(CpEntry::Utf8("protectedMethod".to_string())),
                Some(CpEntry::Utf8("I".to_string())),
                Some(CpEntry::Utf8("()V".to_string())),
                Some(CpEntry::Integer(42)), // Invalid utf8 entry to test error path
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(2),
            super_class: CpIndex(4), // non-object superclass
            interfaces: vec![],
            fields: vec![
                FieldInfo {
                    access_flags: FieldAccessFlags::PROTECTED,
                    name_index: CpIndex(5),
                    descriptor_index: CpIndex(7),
                    attributes: vec![],
                },
                FieldInfo {
                    access_flags: FieldAccessFlags::empty(),
                    name_index: CpIndex(5),
                    descriptor_index: CpIndex(7),
                    attributes: vec![],
                },
                FieldInfo {
                    access_flags: FieldAccessFlags::PUBLIC,
                    name_index: CpIndex(9), // points to Integer to test invalid utf8 fallback
                    descriptor_index: CpIndex(7),
                    attributes: vec![],
                },
            ],
            methods: vec![
                MethodInfo {
                    access_flags: MethodAccessFlags::PROTECTED,
                    name_index: CpIndex(6),
                    descriptor_index: CpIndex(8),
                    attributes: vec![],
                },
                MethodInfo {
                    access_flags: MethodAccessFlags::empty(),
                    name_index: CpIndex(6),
                    descriptor_index: CpIndex(8),
                    attributes: vec![],
                },
                MethodInfo {
                    access_flags: MethodAccessFlags::PRIVATE,
                    name_index: CpIndex(9), // points to Integer to test invalid utf8 fallback
                    descriptor_index: CpIndex(8),
                    attributes: vec![],
                },
            ],
            attributes: vec![],
        };

        let uml = generate_mermaid_uml(&cf);
        assert!(uml.contains("classDiagram"));
        assert!(uml.contains("class ValidClass {"));
        assert!(uml.contains("#I protectedField"));
        assert!(uml.contains("~I protectedField"));
        assert!(uml.contains("+I <invalid>"));
        assert!(uml.contains("#protectedMethod()V"));
        assert!(uml.contains("~protectedMethod()V"));
        assert!(uml.contains("-<invalid>()V"));
        assert!(uml.contains("SuperClass <|-- ValidClass"));
    }

    #[test]
    fn test_resolve_class_name_invalid_paths() {
        let cf = ClassFile {
            minor_version: 0,
            major_version: 52,
            constant_pool: vec![
                None,
                Some(CpEntry::Utf8("ValidClass".to_string())),
                Some(CpEntry::Class {
                    name_index: CpIndex(1),
                }),
                Some(CpEntry::Integer(42)),
                Some(CpEntry::Class {
                    name_index: CpIndex(3),
                }),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(0),       // tests "<none>"
            super_class: CpIndex(3),      // tests "<not a class ref>" (points to integer)
            interfaces: vec![CpIndex(4)], // tests "<invalid utf8>" (class points to integer)
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };

        let uml = generate_mermaid_uml(&cf);
        assert!(uml.contains("class <none> {"));
        assert!(uml.contains("<not a class ref> <|-- <none>"));
        assert!(uml.contains("<invalid utf8> <|.. <none>"));
    }
}
