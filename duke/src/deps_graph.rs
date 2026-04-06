use duke_classfile::{
    ClassFile,
    types::{CpEntry, CpIndex},
};

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

use std::fmt::Write;

pub fn generate_deps_graph(cf: &ClassFile) -> String {
    let mut out = String::new();
    out.push_str("graph TD;\n");
    let this_name = resolve_class_name(cf, cf.this_class);

    for (i, entry) in cf.constant_pool.iter().enumerate() {
        if let Some(CpEntry::Class { .. }) = entry {
            #[allow(clippy::cast_possible_truncation)]
            let idx = CpIndex(i as u16);
            if idx != cf.this_class {
                let ref_name = resolve_class_name(cf, idx);
                if ref_name != "<invalid utf8>" && ref_name != "<not a class ref>" {
                    let safe_this = this_name.replace('/', "_");
                    let safe_ref = ref_name.replace('/', "_");
                    let _ = writeln!(out, "    {safe_this} --> {safe_ref};");
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use duke_classfile::access_flags::ClassAccessFlags;

    #[test]
    fn test_generate_deps_graph() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None,
                Some(CpEntry::Utf8("MyClass".to_string())),
                Some(CpEntry::Class {
                    name_index: CpIndex(1),
                }),
                Some(CpEntry::Utf8("java/lang/Object".to_string())),
                Some(CpEntry::Class {
                    name_index: CpIndex(3),
                }),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(2),
            super_class: CpIndex(4),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };
        let graph = generate_deps_graph(&cf);
        assert!(graph.contains("graph TD"));
        assert!(graph.contains("MyClass --> java_lang_Object"));
    }
}
