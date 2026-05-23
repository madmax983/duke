use duke_classfile::{
    ClassFile, {CpEntry, CpIndex},
};

#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
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

#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
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

pub struct RiskRule {
    pub class_name: &'static str,
    pub method_name: &'static str,
    pub severity: &'static str,
    pub description: &'static str,
}

pub const DEFAULT_RULES: &[RiskRule] = &[
    RiskRule {
        class_name: "java/lang/Runtime",
        method_name: "exec",
        severity: "CRITICAL",
        description: "Executes an arbitrary OS command.",
    },
    RiskRule {
        class_name: "java/lang/ProcessBuilder",
        method_name: "start",
        severity: "CRITICAL",
        description: "Starts a new OS process.",
    },
    RiskRule {
        class_name: "javax/naming/InitialContext",
        method_name: "lookup",
        severity: "HIGH",
        description: "Performs JNDI lookup, potentially vulnerable to Log4Shell-style attacks.",
    },
    RiskRule {
        class_name: "java/io/ObjectInputStream",
        method_name: "readObject",
        severity: "HIGH",
        description: "Deserializes Java objects, potentially leading to RCE.",
    },
    RiskRule {
        class_name: "java/lang/System",
        method_name: "loadLibrary",
        severity: "MEDIUM",
        description: "Loads a native library, which can execute arbitrary native code.",
    },
    RiskRule {
        class_name: "java/net/URLClassLoader",
        method_name: "<init>",
        severity: "MEDIUM",
        description: "Instantiates a class loader from URLs, potentially loading remote code.",
    },
];

pub struct RiskMatch {
    pub rule: &'static RiskRule,
    pub location: String, // E.g. "Methodref #10" or something more descriptive
}

#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
pub fn scan_classfile(cf: &ClassFile) -> Vec<RiskMatch> {
    let mut matches = Vec::new();

    for (i, slot) in cf.constant_pool.iter().enumerate() {
        if let Some(
            CpEntry::Methodref {
                class_index,
                name_and_type_index,
            }
            | CpEntry::InterfaceMethodref {
                class_index,
                name_and_type_index,
            },
        ) = slot
        {
            let target_class = resolve_class_name(cf, *class_index);

            let name_str = cf
                .constant_pool
                .get(name_and_type_index.0 as usize)
                .and_then(|s| s.as_ref())
                .and_then(|nat| {
                    if let CpEntry::NameAndType { name_index, .. } = nat {
                        cp_str(cf, *name_index)
                    } else {
                        None
                    }
                })
                .unwrap_or("<invalid>");

            for rule in DEFAULT_RULES {
                if rule.class_name == target_class && rule.method_name == name_str {
                    matches.push(RiskMatch {
                        rule,
                        location: format!("Constant Pool #{i}"),
                    });
                }
            }
        }
    }

    matches
}

use duke_classfile::parse;
use duke_loader::{ClassLoader, ZipLoader};
use std::path::Path;
use std::process;

/// Scans a single class file for dangerous APIs and prints a report.
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
pub fn dump_scan(path: &str) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path}': {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });

    let matches = scan_classfile(&cf);

    println!("======================================");
    println!(" Security Scan Report: {path}");
    println!("======================================");
    if matches.is_empty() {
        println!("✅ No known vulnerable APIs detected.");
    } else {
        println!("⚠️  Found {} potential vulnerabilities:", matches.len());
        for m in matches {
            println!(
                "  [{}] {}::{} ({})",
                m.rule.severity, m.rule.class_name, m.rule.method_name, m.location
            );
            println!("    Reason: {}", m.rule.description);
            println!();
        }
    }
}

/// Scans all class files within a JAR file for dangerous APIs and prints a summary.
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
pub fn dump_jar_scan(jar_path: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    let mut total_classes = 0;
    let mut total_matches = 0;

    println!("======================================");
    println!(" JAR Security Scan Report: {jar_path}");
    println!("======================================");

    let reader = loader.reader();

    for entry_name in reader.entry_names() {
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        if !entry_name.ends_with(".class") {
            continue;
        }
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        #[allow(clippy::collapsible_if)]
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                total_classes += 1;
                let matches = scan_classfile(&cf);
                if !matches.is_empty() {
                    total_matches += matches.len();
                    println!("⚠️  In class '{class_name_internal}':");
                    for m in matches {
                        println!(
                            "    [{}] {}::{} ({})",
                            m.rule.severity, m.rule.class_name, m.rule.method_name, m.location
                        );
                        println!("      Reason: {}", m.rule.description);
                    }
                    println!();
                }
            }
        }
    }

    println!("--------------------------------------");
    println!("Scanned {total_classes} classes.");
    if total_matches == 0 {
        println!("✅ No known vulnerable APIs detected in this JAR.");
    } else {
        println!("🚨 Found {total_matches} total potential vulnerabilities across the JAR.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use duke_classfile::ClassAccessFlags;

    fn create_mock_classfile_with_methodref(class_name: &str, method_name: &str) -> ClassFile {
        ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None,                                        // 0 is reserved
                Some(CpEntry::Utf8(class_name.to_string())), // 1
                Some(CpEntry::Class {
                    name_index: CpIndex(1),
                }), // 2
                Some(CpEntry::Utf8(method_name.to_string())), // 3
                Some(CpEntry::Utf8("()V".to_string())),      // 4
                Some(CpEntry::NameAndType {
                    name_index: CpIndex(3),
                    descriptor_index: CpIndex(4),
                }), // 5
                Some(CpEntry::Methodref {
                    class_index: CpIndex(2),
                    name_and_type_index: CpIndex(5),
                }), // 6
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(0),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        }
    }

    #[test]
    fn test_dump_scan_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/HelloWorld.class");
        let path = path.to_str().unwrap();

        super::dump_scan(path);
    }

    #[test]
    fn test_dump_jar_scan_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path = path.to_str().unwrap();

        super::dump_jar_scan(path);
    }

    #[test]
    fn test_scan_classfile_detects_runtime_exec() {
        let cf = create_mock_classfile_with_methodref("java/lang/Runtime", "exec");
        let matches = scan_classfile(&cf);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].rule.class_name, "java/lang/Runtime");
        assert_eq!(matches[0].rule.method_name, "exec");
        assert_eq!(matches[0].location, "Constant Pool #6");
    }

    #[test]
    fn test_scan_classfile_ignores_safe_methods() {
        let cf = create_mock_classfile_with_methodref("java/lang/String", "length");
        let matches = scan_classfile(&cf);
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_scan_classfile_invalid_indices_handled() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![
                None, // 0
                Some(CpEntry::Methodref {
                    class_index: CpIndex(10),
                    name_and_type_index: CpIndex(10),
                }), // 1 (invalid indices)
            ],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(0),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };
        let matches = scan_classfile(&cf);
        assert_eq!(matches.len(), 0);
    }
}
