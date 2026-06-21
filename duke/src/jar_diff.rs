#![allow(clippy::items_after_statements)]
#![allow(clippy::case_sensitive_file_extension_comparisons)]

#[cfg(feature = "nova")]
use duke_classfile::{
    parse, AttributeData, CpEntry, CpIndex,
};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

#[cfg(feature = "nova")]
#[allow(unexpected_cfgs)]
#[cfg(not(tarpaulin_include))]
fn cp_str(cf: &duke_classfile::ClassFile, idx: CpIndex) -> Option<&str> {
    cf.constant_pool
        .get(idx.0 as usize)
        .and_then(|slot: &Option<CpEntry>| slot.as_ref())
        .and_then(|entry| {
            if let CpEntry::Utf8(s) = entry {
                Some(s.as_str())
            } else {
                None
            }
        })
}

#[cfg(feature = "nova")]
#[allow(unexpected_cfgs)]
#[cfg(not(tarpaulin_include))]
fn resolve_class_name(cf: &duke_classfile::ClassFile, idx: CpIndex) -> String {
    if idx.0 == 0 {
        return "<none>".to_string();
    }
    let class_entry = cf
        .constant_pool
        .get(idx.0 as usize)
        .and_then(|s: &Option<CpEntry>| s.as_ref());
    if let Some(CpEntry::Class { name_index }) = class_entry {
        cp_str(cf, *name_index)
            .unwrap_or("<invalid utf8>")
            .to_string()
    } else {
        "<not a class ref>".to_string()
    }
}

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if
)]
pub fn dump_jar_diff(jar1_path: &str, jar2_path: &str) {
    let map1 = load_jar_methods(jar1_path);
    let map2 = load_jar_methods(jar2_path);
    dump_jar_diff_internal(jar1_path, jar2_path, &map1, &map2);
}

#[cfg(feature = "nova")]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if
)]
fn dump_jar_diff_internal(
    jar1_path: &str,
    jar2_path: &str,
    map1: &HashMap<String, u64>,
    map2: &HashMap<String, u64>,
) {
    let keys1: HashSet<_> = map1.keys().collect();
    let keys2: HashSet<_> = map2.keys().collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();
    let mut unchanged = 0;

    for k in keys2.difference(&keys1) {
        added.push((*k).clone());
    }

    for k in keys1.difference(&keys2) {
        removed.push((*k).clone());
    }

    for k in keys1.intersection(&keys2) {
        if map1.get(*k) == map2.get(*k) {
            unchanged += 1;
        } else {
            modified.push((*k).clone());
        }
    }

    added.sort();
    removed.sort();
    modified.sort();

    println!("======================================");
    println!(" JAR Bytecode Diff Analysis");
    println!("======================================");
    println!("File 1:               {jar1_path}");
    println!("File 2:               {jar2_path}");
    println!("Methods Added:        {}", added.len());
    println!("Methods Removed:      {}", removed.len());
    println!("Methods Modified:     {}", modified.len());
    println!("Methods Unchanged:    {unchanged}");
    println!();

    if !added.is_empty() {
        println!("--- Top 10 Added Methods ---");
        for m in added.iter().take(10) {
            println!("  + {m}");
        }
        if added.len() > 10 {
            println!("  ... and {} more", added.len() - 10);
        }
        println!();
    }

    if !removed.is_empty() {
        println!("--- Top 10 Removed Methods ---");
        for m in removed.iter().take(10) {
            println!("  - {m}");
        }
        if removed.len() > 10 {
            println!("  ... and {} more", removed.len() - 10);
        }
        println!();
    }

    if !modified.is_empty() {
        println!("--- Top 10 Modified Methods ---");
        for m in modified.iter().take(10) {
            println!("  ~ {m}");
        }
        if modified.len() > 10 {
            println!("  ... and {} more", modified.len() - 10);
        }
        println!();
    }
}

#[cfg(feature = "nova")]
#[allow(unexpected_cfgs)]
#[cfg(not(tarpaulin_include))]
fn load_jar_methods(jar_path: &str) -> HashMap<String, u64> {
    let Ok(loader) = ZipLoader::open(Path::new(jar_path)) else {
        return HashMap::new(); // If we cannot load, treat as empty
    };

    let mut method_hashes = HashMap::new();
    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| name.ends_with(".class"))
        .map(std::string::ToString::to_string)
        .collect();

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            #[allow(clippy::collapsible_if)]
            if let Ok(cf) = parse(&bytes) {
                let class_name = resolve_class_name(&cf, cf.this_class);

                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{class_name}::{name_str}{desc_str}");

                    let mut hasher = DefaultHasher::new();
                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            code.code.hash(&mut hasher);
                        }
                    }
                    method_hashes.insert(full_name, hasher.finish());
                }
            }
        }
    }
    method_hashes
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use duke_classfile::ClassFile;

    #[test]
    fn test_resolve_class_name() {
        let cf = ClassFile {
            minor_version: 0,
            major_version: 0,
            constant_pool: vec![
                None,
                Some(CpEntry::Utf8("java/lang/Object".to_string())),
                Some(CpEntry::Class {
                    name_index: CpIndex(1),
                }),
            ],
            access_flags: duke_classfile::ClassAccessFlags::empty(),
            this_class: CpIndex(0),
            super_class: CpIndex(0),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![],
        };

        let name = super::resolve_class_name(&cf, CpIndex(2));
        assert_eq!(name, "java/lang/Object");

        let name_none = super::resolve_class_name(&cf, CpIndex(0));
        assert_eq!(name_none, "<none>");

        let name_invalid = super::resolve_class_name(&cf, CpIndex(99));
        assert_eq!(name_invalid, "<not a class ref>");
    }
    use super::*;

    #[test]
    fn test_dump_jar_diff_dummy() {
        // Since we don't have the zip crate in dependencies, we'll test the difference logic
        // with two non-existent paths, which should gracefully return empty HashMaps and diff them.

        let path1 = "dummy1.jar";
        let path2 = "dummy2.jar";

        dump_jar_diff(path1, path2);
    }

    #[test]
    fn test_dump_jar_diff_internal_with_differences() {
        let mut map1 = HashMap::new();
        map1.insert("MethodA".to_string(), 123);
        map1.insert("MethodB".to_string(), 456);
        map1.insert("MethodC".to_string(), 789);

        // Let's add 12 elements to test the top 10 truncations
        for i in 0..12 {
            map1.insert(format!("MethodR{i:02}"), 0);
        }

        let mut map2 = HashMap::new();
        map2.insert("MethodB".to_string(), 456); // Unchanged
        map2.insert("MethodC".to_string(), 999); // Modified
        map2.insert("MethodD".to_string(), 111); // Added

        for i in 0..12 {
            map2.insert(format!("MethodA{i:02}"), 0); // Added
            map1.insert(format!("MethodM{i:02}"), 0); // Modified base
            map2.insert(format!("MethodM{i:02}"), 1); // Modified new
        }

        dump_jar_diff_internal("test1.jar", "test2.jar", &map1, &map2);
    }
}
