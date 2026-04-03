//! UML Class Diagram Generation.
//!
//! Generates a Mermaid JS `classDiagram` from a parsed `ClassFile`.

use duke_classfile::{
    ClassFile,
    access_flags::{FieldAccessFlags, MethodAccessFlags},
    types::{CpEntry, CpIndex},
};
use std::fmt::Write;

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

const fn format_visibility_field(flags: FieldAccessFlags) -> &'static str {
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

const fn format_visibility_method(flags: MethodAccessFlags) -> &'static str {
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

fn simplify_descriptor(desc: &str) -> String {
    // A simplistic replacement for MVP.
    // Just replace "/" with "."
    desc.replace('/', ".")
}

/// Generates a Mermaid JS `classDiagram` from the given `ClassFile`.
#[must_use]
pub fn generate_mermaid_uml(cf: &ClassFile) -> String {
    let mut out = String::from("classDiagram\n");

    let class_name = resolve_class_name(cf, cf.this_class).replace('/', ".");
    let super_name = resolve_class_name(cf, cf.super_class).replace('/', ".");

    let _ = writeln!(out, "    class {class_name} {{");

    for field in &cf.fields {
        let name = cp_str(cf, field.name_index).unwrap_or("<invalid>");
        let desc = cp_str(cf, field.descriptor_index).unwrap_or("<invalid>");
        let vis = format_visibility_field(field.access_flags);

        let static_mod = if field.access_flags.contains(FieldAccessFlags::STATIC) {
            "$"
        } else {
            ""
        };
        let _ = writeln!(
            out,
            "        {}{}{} {}",
            vis,
            static_mod,
            simplify_descriptor(desc),
            name
        );
    }

    for method in &cf.methods {
        let name = cp_str(cf, method.name_index).unwrap_or("<invalid>");
        let desc = cp_str(cf, method.descriptor_index).unwrap_or("<invalid>");
        let vis = format_visibility_method(method.access_flags);

        let static_mod = if method.access_flags.contains(MethodAccessFlags::STATIC) {
            "$"
        } else {
            ""
        };
        let abstract_mod = if method.access_flags.contains(MethodAccessFlags::ABSTRACT) {
            "*"
        } else {
            ""
        };

        let _ = writeln!(
            out,
            "        {}{}{}{} {}",
            vis,
            static_mod,
            abstract_mod,
            name,
            simplify_descriptor(desc)
        );
    }

    let _ = writeln!(out, "    }}");

    if super_name != "<none>" && super_name != "java.lang.Object" {
        let _ = writeln!(out, "    {class_name} --|> {super_name}");
    }

    for &iface_idx in &cf.interfaces {
        let iface_name = resolve_class_name(cf, iface_idx).replace('/', ".");
        let _ = writeln!(out, "    {class_name} ..|> {iface_name}");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use duke_classfile::access_flags::ClassAccessFlags;
    use duke_classfile::types::FieldInfo;
    use duke_classfile::types::MethodInfo;

    #[test]
    fn test_generate_mermaid_uml() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None, // 0
                Some(CpEntry::Class {
                    name_index: CpIndex(2),
                }), // 1
                Some(CpEntry::Utf8("com/example/MyClass".to_string())), // 2
                Some(CpEntry::Class {
                    name_index: CpIndex(4),
                }), // 3
                Some(CpEntry::Utf8("java/lang/Object".to_string())), // 4
                Some(CpEntry::Class {
                    name_index: CpIndex(6),
                }), // 5
                Some(CpEntry::Utf8("java/io/Serializable".to_string())), // 6
                Some(CpEntry::Utf8("myField".to_string())), // 7
                Some(CpEntry::Utf8("I".to_string())), // 8
                Some(CpEntry::Utf8("myMethod".to_string())), // 9
                Some(CpEntry::Utf8("()V".to_string())), // 10
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(1),
            super_class: CpIndex(3),
            interfaces: vec![CpIndex(5)],
            fields: vec![FieldInfo {
                access_flags: FieldAccessFlags::PRIVATE | FieldAccessFlags::STATIC,
                name_index: CpIndex(7),
                descriptor_index: CpIndex(8),
                attributes: vec![],
            }],
            methods: vec![MethodInfo {
                access_flags: MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT,
                name_index: CpIndex(9),
                descriptor_index: CpIndex(10),
                attributes: vec![],
            }],
            attributes: vec![],
        };

        let uml = generate_mermaid_uml(&cf);
        assert!(uml.contains("classDiagram"));
        assert!(uml.contains("class com.example.MyClass {"));
        assert!(uml.contains("-$I myField"));
        assert!(uml.contains("+*myMethod ()V"));
        assert!(uml.contains("com.example.MyClass ..|> java.io.Serializable"));
        // Object should be omitted as super class
        assert!(!uml.contains("com.example.MyClass --|> java.lang.Object"));
    }
}
