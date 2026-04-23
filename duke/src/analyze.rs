use duke_bytecode::{cyclomatic_complexity, decode};
use duke_classfile::{
    ClassFile,
    types::{AttributeData, CpEntry, CpIndex},
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

/// Generates a static analysis report for the given class file.
///
/// **Why it exists:** Provides a quick, text-based overview of a specific class's
/// internal complexity. Instead of guessing which method is the most convoluted,
/// this generates a clean table of code sizes and cyclomatic complexities to
/// guide refactoring efforts.
///
/// Iterates over all methods, calculating code size and cyclomatic complexity.
///
/// # Examples
///
/// ```ignore
/// use duke::analyze::generate_analysis_report;
/// use duke_classfile::ClassFile;
///
/// let cf: ClassFile = get_parsed_class_somehow();
/// let report_string = generate_analysis_report(&cf);
/// println!("{}", report_string);
/// ```
#[must_use]
pub fn generate_analysis_report(cf: &ClassFile) -> String {
    let mut out = String::new();
    let class_name = resolve_class_name(cf, cf.this_class);

    let _ = writeln!(&mut out, "=== Static Analysis Report: {class_name} ===");
    let _ = writeln!(
        &mut out,
        "{:<35} | {:<10} | {:<10}",
        "Method", "Complexity", "Code Size"
    );
    let _ = writeln!(&mut out, "{}", "-".repeat(61));

    for method in &cf.methods {
        let name_str = cp_str(cf, method.name_index).unwrap_or("<invalid>");
        let desc_str = cp_str(cf, method.descriptor_index).unwrap_or("<invalid>");
        let full_name = format!("{name_str}{desc_str}");

        let mut found_code = false;
        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data {
                found_code = true;
                let code_size = code.code.len();
                match decode(&code.code) {
                    Ok(instructions) => {
                        let complexity = cyclomatic_complexity(&instructions);
                        let _ = writeln!(
                            &mut out,
                            "{full_name:<35} | {complexity:<10} | {code_size:<10}"
                        );
                    }
                    Err(_) => {
                        let _ = writeln!(
                            &mut out,
                            "{full_name:<35} | {:<10} | {code_size:<10}",
                            "Error"
                        );
                    }
                }
            }
        }

        if !found_code {
            let _ = writeln!(&mut out, "{full_name:<35} | {:<10} | {:<10}", "N/A", "0");
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use duke_classfile::types::{AttributeInfo, CodeAttribute, MethodInfo};
    use duke_classfile::{ClassAccessFlags, MethodAccessFlags};

    #[test]
    fn test_dump_analyze_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/HelloWorld.class");
        let path = path.to_str().unwrap();

        let bytes = std::fs::read(path).unwrap();
        let cf = duke_classfile::parse(&bytes).unwrap();
        let report = super::generate_analysis_report(&cf);
        assert!(report.contains("HelloWorld"));
    }

    #[test]
    fn test_generate_analysis_report_empty() {
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

        let report = generate_analysis_report(&cf);
        assert!(report.contains("Static Analysis Report: <none>"));
        assert!(report.contains("Method"));
        assert!(report.contains("Complexity"));
        assert!(report.contains("Code Size"));
    }

    #[test]
    fn test_generate_analysis_report_with_methods() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None,
                Some(CpEntry::Class {
                    name_index: CpIndex(2),
                }),
                Some(CpEntry::Utf8("MyClass".to_string())),
                Some(CpEntry::Utf8("myMethod".to_string())),
                Some(CpEntry::Utf8("()V".to_string())),
                Some(CpEntry::Utf8("abstractMethod".to_string())),
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
                            code: vec![0xb1], // return
                            exception_table: vec![],
                            attributes: vec![],
                        }),
                    }],
                },
                MethodInfo {
                    access_flags: MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT,
                    name_index: CpIndex(5),
                    descriptor_index: CpIndex(4),
                    attributes: vec![],
                },
                MethodInfo {
                    access_flags: MethodAccessFlags::PUBLIC,
                    name_index: CpIndex(6),
                    descriptor_index: CpIndex(4),
                    attributes: vec![AttributeInfo {
                        name_index: CpIndex(0),
                        data: AttributeData::Code(CodeAttribute {
                            max_stack: 1,
                            max_locals: 1,
                            code: vec![0xFE], // Invalid opcode IMPDEP1 to trigger Err path
                            exception_table: vec![],
                            attributes: vec![],
                        }),
                    }],
                },
            ],
            attributes: vec![],
        };

        // Add the extra constants
        let mut cf = cf;
        cf.constant_pool
            .push(Some(CpEntry::Utf8("invalidMethod".to_string())));

        let report = generate_analysis_report(&cf);
        assert!(report.contains("Static Analysis Report: MyClass"));
        assert!(report.contains("myMethod()V"));
        assert!(report.contains("abstractMethod()V"));
        assert!(report.contains("invalidMethod()V"));
        assert!(report.contains("N/A")); // for abstract method
        assert!(report.contains("Error")); // for decoding error
        assert!(report.contains('1')); // code size for return
        assert!(report.contains('1')); // complexity for return
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

        let report = generate_analysis_report(&cf);
        assert!(report.contains("Static Analysis Report: <not a class ref>"));
    }

    #[test]
    fn test_resolve_class_name_invalid_utf8_index() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None,
                Some(CpEntry::Class {
                    name_index: CpIndex(2),
                }),
                Some(CpEntry::Integer(42)), // Not a Utf8 entry
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(1),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };

        let report = generate_analysis_report(&cf);
        assert!(report.contains("Static Analysis Report: <invalid utf8>"));
    }
}
