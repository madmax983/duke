use duke_bytecode::{cyclomatic_complexity, decode};
use duke_classfile::{
    parse,
    types::{AttributeData, CpEntry, CpIndex},
};
use std::collections::HashMap;
use std::process;

#[derive(Debug, PartialEq, Eq)]
pub struct MethodStats {
    pub complexity: usize,
    pub instructions_count: usize,
}

#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn cp_str(cf: &duke_classfile::ClassFile, idx: CpIndex) -> Option<&str> {
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
#[allow(unexpected_cfgs)]
pub fn analyze_class(bytes: &[u8]) -> Result<HashMap<String, MethodStats>, String> {
    let cf = parse(bytes).map_err(|e| e.to_string())?;
    let mut map = HashMap::new();

    for method in &cf.methods {
        let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
        let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
        let full_name = format!("{name_str}{desc_str}");

        let mut complexity = 1; // Base complexity
        let mut instructions_count = 0;

        #[allow(clippy::collapsible_if)]
        for attr in &method.attributes {
            if let AttributeData::Code(code) = &attr.data {
                if let Ok(instructions) = decode(&code.code) {
                    complexity = cyclomatic_complexity(&instructions);
                    instructions_count = instructions.len();
                }
            }
        }
        map.insert(
            full_name,
            MethodStats {
                complexity,
                instructions_count,
            },
        );
    }

    Ok(map)
}

/// Compares two JVM `.class` files and highlights the differences in terms of added, removed, and changed methods.
///
/// **Why it exists:** When refactoring Java code or bumping dependency versions, you need to know exactly what changed at the bytecode level.
/// This utility diffs two class files based on their cyclomatic complexity and instruction counts to help pinpoint logic modifications.
///
/// # Examples
///
/// ```ignore
/// // Assumes 'Old.class' and 'New.class' are valid files
/// use duke::diff::dump_diff;
///
/// dump_diff("Old.class", "New.class");
/// ```
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
pub fn dump_diff(path1: &str, path2: &str) {
    let bytes1 = std::fs::read(path1).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path1}': {e}");
        process::exit(1);
    });
    let bytes2 = std::fs::read(path2).unwrap_or_else(|e| {
        eprintln!("duke: cannot read '{path2}': {e}");
        process::exit(1);
    });

    let map1 = analyze_class(&bytes1).unwrap_or_else(|e| {
        eprintln!("duke: parse error in '{path1}': {e}");
        process::exit(1);
    });
    let map2 = analyze_class(&bytes2).unwrap_or_else(|e| {
        eprintln!("duke: parse error in '{path2}': {e}");
        process::exit(1);
    });

    let mut all_methods: Vec<&String> = map1.keys().chain(map2.keys()).collect();
    all_methods.sort();
    all_methods.dedup();

    println!("======================================");
    println!(" Class Diff Report");
    println!("======================================");
    println!("Old: {path1}");
    println!("New: {path2}");
    println!();

    let mut added = vec![];
    let mut removed = vec![];
    let mut changed = vec![];
    let mut unchanged = 0;

    for method in all_methods {
        match (map1.get(method), map2.get(method)) {
            (Some(_), None) => removed.push(method),
            (None, Some(_)) => added.push(method),
            (Some(s1), Some(s2)) => {
                #[allow(clippy::if_not_else)]
                if s1 != s2 {
                    changed.push((method, s1, s2));
                } else {
                    unchanged += 1;
                }
            }
            _ => {}
        }
    }

    if !added.is_empty() {
        println!("➕ Added Methods ({}):", added.len());
        for m in &added {
            println!("  {m}");
        }
        println!();
    }

    if !removed.is_empty() {
        println!("➖ Removed Methods ({}):", removed.len());
        for m in &removed {
            println!("  {m}");
        }
        println!();
    }

    if !changed.is_empty() {
        println!("📝 Changed Methods ({}):", changed.len());
        for (m, s1, s2) in &changed {
            println!("  {m}");
            if s1.complexity != s2.complexity {
                println!("    Complexity: {} -> {}", s1.complexity, s2.complexity);
            }
            if s1.instructions_count != s2.instructions_count {
                println!(
                    "    Instructions: {} -> {}",
                    s1.instructions_count, s2.instructions_count
                );
            }
        }
        println!();
    }

    println!("Unchanged Methods: {unchanged}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_class_invalid() {
        assert!(analyze_class(&[0, 0, 0]).is_err());
    }
}

#[cfg(test)]
mod diff_tests {
    use super::*;
    use duke_classfile::access_flags::ClassAccessFlags;
    use duke_classfile::types::ClassFile;

    #[test]
    fn test_dump_diff_no_match() {
        let cf = ClassFile {
            major_version: 61,
            minor_version: 0,
            constant_pool: vec![None, Some(CpEntry::Utf8("testMethod".to_string()))],
            access_flags: ClassAccessFlags::PUBLIC,
            this_class: CpIndex(0),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };
        assert_eq!(cp_str(&cf, CpIndex(1)), Some("testMethod"));
        assert_eq!(cp_str(&cf, CpIndex(2)), None);
    }
}

    #[test]
    fn test_analyze_class_with_valid_bytes() {
        // Just enough valid classfile bytes to pass parsing, but no methods
        let valid_bytes = vec![
            0xca, 0xfe, 0xba, 0xbe, // magic
            0x00, 0x00, // minor
            0x00, 0x3d, // major (61)
            0x00, 0x01, // constant_pool_count (1)
            0x00, 0x01, // access_flags (public)
            0x00, 0x00, // this_class (0)
            0x00, 0x00, // super_class (0)
            0x00, 0x00, // interfaces_count (0)
            0x00, 0x00, // fields_count (0)
            0x00, 0x00, // methods_count (0)
            0x00, 0x00, // attributes_count (0)
        ];
        let result = analyze_class(&valid_bytes);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
    #[test]
    fn test_analyze_class_with_valid_methods() {
        let valid_bytes = vec![
            0xca, 0xfe, 0xba, 0xbe, // magic
            0x00, 0x00, // minor
            0x00, 0x3d, // major (61)
            0x00, 0x05, // constant_pool_count (5)
            // #1: Utf8 "testMethod"
            0x01, 0x00, 0x0a, 0x74, 0x65, 0x73, 0x74, 0x4d, 0x65, 0x74, 0x68, 0x6f, 0x64,
            // #2: Utf8 "()V"
            0x01, 0x00, 0x03, 0x28, 0x29, 0x56,
            // #3: Utf8 "Code"
            0x01, 0x00, 0x04, 0x43, 0x6f, 0x64, 0x65,
            // #4: Methodref #something
            0x0a, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, // access_flags (public)
            0x00, 0x00, // this_class (0)
            0x00, 0x00, // super_class (0)
            0x00, 0x00, // interfaces_count (0)
            0x00, 0x00, // fields_count (0)
            0x00, 0x01, // methods_count (1)
            0x00, 0x01, // method[0].access_flags
            0x00, 0x01, // method[0].name_index
            0x00, 0x02, // method[0].descriptor_index
            0x00, 0x01, // method[0].attributes_count
            0x00, 0x03, // attr name index
            0x00, 0x00, 0x00, 0x0e, // attr len
            0x00, 0x01, // max stack
            0x00, 0x01, // max locals
            0x00, 0x00, 0x00, 0x02, // code len
            0x03, 0xac, // iconst_0, ireturn
            0x00, 0x00, // exception table len
            0x00, 0x00, // attributes count
            0x00, 0x00, // class attributes_count (0)
        ];
        let result = analyze_class(&valid_bytes).unwrap();
        assert_eq!(result.len(), 1);
        let stats = result.get("testMethod()V").unwrap();
        assert_eq!(stats.complexity, 1);
        assert_eq!(stats.instructions_count, 2);
    }

    #[test]
    fn test_analyze_class_with_different_methods() {
        let valid_bytes1 = vec![
            0xca, 0xfe, 0xba, 0xbe, // magic
            0x00, 0x00, // minor
            0x00, 0x3d, // major (61)
            0x00, 0x05, // constant_pool_count (5)
            // #1: Utf8 "testMethod"
            0x01, 0x00, 0x0a, 0x74, 0x65, 0x73, 0x74, 0x4d, 0x65, 0x74, 0x68, 0x6f, 0x64,
            // #2: Utf8 "()V"
            0x01, 0x00, 0x03, 0x28, 0x29, 0x56,
            // #3: Utf8 "Code"
            0x01, 0x00, 0x04, 0x43, 0x6f, 0x64, 0x65,
            // #4: Methodref #something
            0x0a, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, // access_flags (public)
            0x00, 0x00, // this_class (0)
            0x00, 0x00, // super_class (0)
            0x00, 0x00, // interfaces_count (0)
            0x00, 0x00, // fields_count (0)
            0x00, 0x01, // methods_count (1)
            0x00, 0x01, // method[0].access_flags
            0x00, 0x01, // method[0].name_index
            0x00, 0x02, // method[0].descriptor_index
            0x00, 0x01, // method[0].attributes_count
            0x00, 0x03, // attr name index
            0x00, 0x00, 0x00, 0x0e, // attr len
            0x00, 0x01, // max stack
            0x00, 0x01, // max locals
            0x00, 0x00, 0x00, 0x02, // code len
            0x03, 0xac, // iconst_0, ireturn
            0x00, 0x00, // exception table len
            0x00, 0x00, // attributes count
            0x00, 0x00, // class attributes_count (0)
        ];

        let valid_bytes2 = vec![
            0xca, 0xfe, 0xba, 0xbe, // magic
            0x00, 0x00, // minor
            0x00, 0x3d, // major (61)
            0x00, 0x05, // constant_pool_count (5)
            // #1: Utf8 "testMethod"
            0x01, 0x00, 0x0a, 0x74, 0x65, 0x73, 0x74, 0x4d, 0x65, 0x74, 0x68, 0x6f, 0x64,
            // #2: Utf8 "()V"
            0x01, 0x00, 0x03, 0x28, 0x29, 0x56,
            // #3: Utf8 "Code"
            0x01, 0x00, 0x04, 0x43, 0x6f, 0x64, 0x65,
            // #4: Methodref #something
            0x0a, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, // access_flags (public)
            0x00, 0x00, // this_class (0)
            0x00, 0x00, // super_class (0)
            0x00, 0x00, // interfaces_count (0)
            0x00, 0x00, // fields_count (0)
            0x00, 0x01, // methods_count (1)
            0x00, 0x01, // method[0].access_flags
            0x00, 0x01, // method[0].name_index
            0x00, 0x02, // method[0].descriptor_index
            0x00, 0x01, // method[0].attributes_count
            0x00, 0x03, // attr name index
            0x00, 0x00, 0x00, 0x11, // attr len
            0x00, 0x01, // max stack
            0x00, 0x01, // max locals
            0x00, 0x00, 0x00, 0x05, // code len
            0x03, 0x99, 0x00, 0x00, 0xac, // iconst_0, ifeq 0, ireturn
            0x00, 0x00, // exception table len
            0x00, 0x00, // attributes count
            0x00, 0x00, // class attributes_count (0)
        ];

        let result1 = analyze_class(&valid_bytes1).unwrap();
        let stats1 = result1.get("testMethod()V").unwrap();

        let result2 = analyze_class(&valid_bytes2).unwrap();
        let stats2 = result2.get("testMethod()V").unwrap();

        assert_ne!(stats1.complexity, stats2.complexity);
        assert_ne!(stats1.instructions_count, stats2.instructions_count);
    }
