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

/// Generates a Mermaid `graph TD` of class dependencies from the constant pool.
///
/// **Why it exists:** To truly understand a class, you must understand its dependencies.
/// This function reveals the invisible web of connections between classes by extracting
/// all class references from the constant pool, providing immediate architectural insight.
///
/// This visualises which other classes the given `ClassFile` refers to, which
/// helps in understanding coupling and architecture.
///
/// # Examples
///
/// ```ignore
/// use duke::deps_graph::generate_deps_graph;
/// use duke_classfile::ClassFile;
///
/// let cf: ClassFile = get_parsed_class_somehow();
/// let graph_string = generate_deps_graph(&cf);
/// println!("{}", graph_string);
/// ```
#[must_use]
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
    use duke_classfile::ClassAccessFlags;

    #[test]
    fn test_cp_str_none() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![None, Some(CpEntry::Integer(42))],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(1),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };
        assert_eq!(super::cp_str(&cf, CpIndex(1)), None);
        assert_eq!(super::cp_str(&cf, CpIndex(0)), None);
    }

    #[test]
    fn test_resolve_class_name_invalid_index() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![None], // Missing class entry
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(1), // Points to out-of-bounds or non-class
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };
        assert_eq!(
            super::resolve_class_name(&cf, CpIndex(1)),
            "<not a class ref>"
        );
        assert_eq!(super::resolve_class_name(&cf, CpIndex(0)), "<none>");
    }

    #[test]
    fn test_resolve_class_name_invalid_utf8() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None,
                Some(CpEntry::Class {
                    name_index: CpIndex(2),
                }),
                Some(CpEntry::Integer(42)),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(1),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };
        assert_eq!(super::resolve_class_name(&cf, CpIndex(1)), "<invalid utf8>");
    }

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
