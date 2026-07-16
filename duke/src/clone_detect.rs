use duke_classfile::{ClassFile, CpEntry, CpIndex};
use std::fmt::Write;

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
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

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn resolve_class_name(cf: &ClassFile, idx: CpIndex) -> String {
    if idx.0 == 0 {
        return "<none>".to_string();
    }
    let class_entry = cf
        .constant_pool
        .get(idx.0 as usize)
        .and_then(|s| s.as_ref());
    if let Some(CpEntry::Class { name_index }) = class_entry {
        cp_str(cf, *name_index)
            .unwrap_or("<invalid utf8>")
            .to_string()
    } else {
        "<not a class ref>".to_string()
    }
}

use duke_bytecode::{calculate_similarity, decode};
use duke_classfile::AttributeData;

#[cfg(feature = "nova")]
#[must_use]
#[allow(clippy::collapsible_if)]
pub fn generate_clone_report(cf: &duke_classfile::ClassFile, threshold: f64) -> String {
    let mut out = String::new();
    let class_name = resolve_class_name(cf, cf.this_class);

    let _ = writeln!(&mut out, "=== Clone Detection Report: {class_name} ===");
    let _ = writeln!(&mut out, "Threshold: {threshold:.2}");
    let _ = writeln!(
        &mut out,
        "-------------------------------------------------------------"
    );

    let mut parsed_methods = Vec::new();
    for (idx, method) in cf.methods.iter().enumerate() {
        let name_str = cp_str(cf, method.name_index).unwrap_or("<invalid>");
        let desc_str = cp_str(cf, method.descriptor_index).unwrap_or("<invalid>");
        let full_name = format!("{name_str}{desc_str}");

        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data {
                if let Ok(instructions) = decode(&code.code) {
                    let instrs: Vec<_> = instructions.into_iter().map(|(_, i)| i).collect();
                    parsed_methods.push((idx, full_name.clone(), instrs));
                }
            }
        }
    }

    let mut found_clones = false;
    for i in 0..parsed_methods.len() {
        for j in (i + 1)..parsed_methods.len() {
            let (_, name1, instrs1) = &parsed_methods[i];
            let (_, name2, instrs2) = &parsed_methods[j];

            let sim = calculate_similarity(instrs1, instrs2);
            if sim >= threshold {
                found_clones = true;
                let _ = writeln!(&mut out, "Similarity: {sim:.2}");
                let _ = writeln!(&mut out, "  - {name1}");
                let _ = writeln!(&mut out, "  - {name2}");
                let _ = writeln!(&mut out);
            }
        }
    }

    if !found_clones {
        let _ = writeln!(&mut out, "No clones found.");
    }

    out
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use duke_classfile::{
        AttributeData, AttributeInfo, ClassAccessFlags, CodeAttribute, MethodAccessFlags,
        MethodInfo,
    };

    #[test]
    fn test_generate_clone_report_finds_clones() {
        let mut cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None,
                Some(CpEntry::Utf8("MyClass".to_string())),
                Some(CpEntry::Class {
                    name_index: CpIndex(1),
                }),
                Some(CpEntry::Utf8("methodOne".to_string())),
                Some(CpEntry::Utf8("()V".to_string())),
                Some(CpEntry::Utf8("methodTwo".to_string())),
                Some(CpEntry::Utf8("Code".to_string())),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(2),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };

        let method1 = MethodInfo {
            access_flags: MethodAccessFlags::PUBLIC,
            name_index: CpIndex(3),
            descriptor_index: CpIndex(4),
            attributes: vec![AttributeInfo {
                name_index: CpIndex(6),
                data: AttributeData::Code(CodeAttribute {
                    max_stack: 1,
                    max_locals: 1,
                    code: vec![0xb1], // return
                    exception_table: vec![],
                    attributes: vec![],
                }),
            }],
        };

        let method2 = MethodInfo {
            access_flags: MethodAccessFlags::PUBLIC,
            name_index: CpIndex(5),
            descriptor_index: CpIndex(4),
            attributes: vec![AttributeInfo {
                name_index: CpIndex(6),
                data: AttributeData::Code(CodeAttribute {
                    max_stack: 1,
                    max_locals: 1,
                    code: vec![0xb1], // return
                    exception_table: vec![],
                    attributes: vec![],
                }),
            }],
        };

        cf.methods.push(method1);
        cf.methods.push(method2);

        let report = generate_clone_report(&cf, 0.9);
        assert!(report.contains("methodOne()V"));
        assert!(report.contains("methodTwo()V"));
        assert!(report.contains("1.00")); // Should have a similarity score of 1.0
    }
}
