use duke_classfile::{
    parse,
    types::{ClassFile, CpEntry, CpIndex},
};
use std::collections::HashSet;
use std::process;

/// Helper to extract string from constant pool
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

/// Helper to resolve a Class entry index to its UTF-8 name.
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

pub fn dump_dependencies(path: &str) {
    if let Err(e) = dump_dependencies_inner(path) {
        eprintln!("duke: {}", e);
        process::exit(1);
    }
}

fn dump_dependencies_inner(path: &str) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read '{path}': {e}"))?;
    let cf = parse(&bytes).map_err(|e| format!("parse error in '{path}': {e}"))?;

    let this_class_name = resolve_class_name(&cf, cf.this_class);

    let mut dependencies = HashSet::new();

    for entry in &cf.constant_pool {
        if let Some(CpEntry::Class { name_index }) = entry {
            let class_name = cp_str(&cf, *name_index).unwrap_or("<invalid>");
            // Exclude the class itself and array types
            if class_name != this_class_name && !class_name.starts_with('[') {
                dependencies.insert(class_name);
            }
        }
    }

    let mut sorted_deps: Vec<_> = dependencies.into_iter().collect();
    sorted_deps.sort_unstable();

    println!("graph TD");
    for dep in sorted_deps {
        println!("    \"{this_class_name}\" --> \"{dep}\"");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dump_dependencies_inner() {
        // Minimal sanity test, we'll verify it parses the test class correctly.
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../tests/fixtures/HelloWorld.class");

        // This should succeed
        assert!(dump_dependencies_inner(p.to_str().unwrap()).is_ok());

        // This should fail reading
        let err1 = dump_dependencies_inner("does_not_exist.class").unwrap_err();
        assert!(err1.contains("cannot read"));

        // This should fail parsing
        let mut bad_p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        bad_p.push("Cargo.toml"); // Not a valid class file
        let err2 = dump_dependencies_inner(bad_p.to_str().unwrap()).unwrap_err();
        assert!(err2.contains("parse error"));
    }

    #[test]
    fn test_resolve_class_name_zero() {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../tests/fixtures/HelloWorld.class");
        let bytes = std::fs::read(&p).unwrap();
        let cf = parse(&bytes).unwrap();
        // Index 0 returns "<none>"
        assert_eq!(resolve_class_name(&cf, CpIndex(0)), "<none>");
        // Index 1 (Methodref) returns "<not a class ref>"
        assert_eq!(resolve_class_name(&cf, CpIndex(1)), "<not a class ref>");
    }

    #[test]
    fn test_cp_str_invalid() {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../tests/fixtures/HelloWorld.class");
        let bytes = std::fs::read(&p).unwrap();
        let cf = parse(&bytes).unwrap();
        // Out of bounds
        assert_eq!(cp_str(&cf, CpIndex(999)), None);
        // Not a utf8 entry
        assert_eq!(cp_str(&cf, CpIndex(1)), None);
    }
}
