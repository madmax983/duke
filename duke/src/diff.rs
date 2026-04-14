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
            (Some(s1), Some(s2)) =>
            {
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
    use duke_classfile::access_flags::ClassAccessFlags;
    use duke_classfile::types::ClassFile;

    #[test]
    fn test_analyze_class_invalid() {
        assert!(analyze_class(&[0, 0, 0]).is_err());
    }

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
