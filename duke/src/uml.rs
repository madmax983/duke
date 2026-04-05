use duke_classfile::{
    ClassFile,
    access_flags::{FieldAccessFlags, MethodAccessFlags},
    types::{CpEntry, CpIndex},
};
use std::fmt::Write;

#[cfg(not(tarpaulin_include))]
#[allow(dead_code)]
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

#[cfg(not(tarpaulin_include))]
#[allow(dead_code)]
fn resolve_class_name(cf: &ClassFile, idx: CpIndex) -> String {
    if idx.0 == 0 {
        return "java_lang_Object".to_string(); // Default if super_class is 0
    }
    let class_entry = cf
        .constant_pool
        .get(idx.0 as usize)
        .and_then(|s| s.as_ref());
    if let Some(CpEntry::Class { name_index }) = class_entry {
        cp_str(cf, *name_index)
            .unwrap_or("<invalid utf8>")
            .replace('/', "_")
    } else {
        "<not a class ref>".to_string()
    }
}

#[must_use]
pub fn generate_mermaid_uml(cf: &ClassFile) -> String {
    let mut uml = String::from("classDiagram\n");
    let this_class = resolve_class_name(cf, cf.this_class);
    let super_class = resolve_class_name(cf, cf.super_class);

    if cf.super_class.0 != 0 {
        let _ = writeln!(uml, "    {super_class} <|-- {this_class}");
    }

    for &iface in &cf.interfaces {
        let iface_name = resolve_class_name(cf, iface);
        let _ = writeln!(uml, "    {iface_name} <|.. {this_class}");
    }

    let _ = writeln!(uml, "    class {this_class} {{");

    for field in &cf.fields {
        let name = cp_str(cf, field.name_index).unwrap_or("?");
        let desc = cp_str(cf, field.descriptor_index).unwrap_or("?");
        let vis = if field.access_flags.contains(FieldAccessFlags::PUBLIC) {
            "+"
        } else if field.access_flags.contains(FieldAccessFlags::PRIVATE) {
            "-"
        } else if field.access_flags.contains(FieldAccessFlags::PROTECTED) {
            "#"
        } else {
            "~"
        };
        let desc_clean = desc.replace(';', "");
        let _ = writeln!(uml, "        {vis}{desc_clean} {name}");
    }

    for method in &cf.methods {
        let name = cp_str(cf, method.name_index).unwrap_or("?");
        let desc = cp_str(cf, method.descriptor_index).unwrap_or("?");
        let vis = if method.access_flags.contains(MethodAccessFlags::PUBLIC) {
            "+"
        } else if method.access_flags.contains(MethodAccessFlags::PRIVATE) {
            "-"
        } else if method.access_flags.contains(MethodAccessFlags::PROTECTED) {
            "#"
        } else {
            "~"
        };
        let desc_clean = desc.replace(';', "");
        let _ = writeln!(uml, "        {vis}{name}() {desc_clean}");
    }

    let _ = writeln!(uml, "    }}");

    uml
}

#[cfg(test)]
mod tests {
    use super::*;
    use duke_classfile::access_flags::ClassAccessFlags;

    #[test]
    fn test_generate_mermaid_uml_basic() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None, // 0
                Some(CpEntry::Class {
                    name_index: CpIndex(2),
                }), // 1
                Some(CpEntry::Utf8("MyClass".to_string())), // 2
                Some(CpEntry::Class {
                    name_index: CpIndex(4),
                }), // 3
                Some(CpEntry::Utf8("java/lang/Object".to_string())), // 4
                Some(CpEntry::Utf8("myField".to_string())), // 5
                Some(CpEntry::Utf8("I".to_string())), // 6
                Some(CpEntry::Utf8("myMethod".to_string())), // 7
                Some(CpEntry::Utf8("()V".to_string())), // 8
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(1),
            super_class: CpIndex(3),
            interfaces: vec![],
            fields: vec![duke_classfile::types::FieldInfo {
                access_flags: FieldAccessFlags::PRIVATE,
                name_index: CpIndex(5),
                descriptor_index: CpIndex(6),
                attributes: vec![],
            }],
            methods: vec![duke_classfile::types::MethodInfo {
                access_flags: MethodAccessFlags::PUBLIC,
                name_index: CpIndex(7),
                descriptor_index: CpIndex(8),
                attributes: vec![],
            }],
            attributes: vec![],
        };

        let uml = generate_mermaid_uml(&cf);
        assert!(uml.contains("classDiagram"));
        assert!(uml.contains("java_lang_Object <|-- MyClass"));
        assert!(uml.contains("-I myField"));
        assert!(uml.contains("+myMethod() ()V"));
    }
}
