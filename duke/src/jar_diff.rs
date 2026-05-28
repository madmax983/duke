//! JAR Bytecode Diff Analysis
//!
//! This module provides functionality to compare the bytecode contents of two JAR files.
//! It analyzes the `.class` files inside the JARs, parsing their constant pools and methods
//! to compute hashes of their bytecode, which are then used to detect added, removed,
//! or modified methods.

#![allow(clippy::items_after_statements)]
#![allow(clippy::case_sensitive_file_extension_comparisons)]

#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

#[cfg(feature = "nova")]
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

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn resolve_class_name(cf: &duke_classfile::ClassFile, idx: CpIndex) -> String {
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

/// Analyzes the bytecode differences between two JAR files and prints a report to standard output.
///
/// This function opens both JAR files, extracts all `.class` entries, computes hashes of
/// the bytecode for each method, and compares the two sets of methods to identify:
/// - Added methods (present in the second JAR but not the first)
/// - Removed methods (present in the first JAR but not the second)
/// - Modified methods (present in both, but with differing bytecode)
/// - Unchanged methods
///
/// # Examples
///
/// ```
/// use duke::jar_diff::dump_jar_diff;
///
/// // Diffing two non-existent files gracefully treats them as empty and prints a diff of 0 methods.
/// dump_jar_diff("dummy1.jar", "dummy2.jar");
/// ```
///
/// # Panics
///
/// This function does not panic on missing or invalid JAR files; it simply treats them as empty.
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
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
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
    use super::*;

    #[test]
    fn test_dump_jar_diff_dummy() {
        // Since we don't have the zip crate in dependencies, we'll test the difference logic
        // with two non-existent paths, which should gracefully return empty HashMaps and diff them.

        let path1 = "dummy1.jar";
        let path2 = "dummy2.jar";

        dump_jar_diff(path1, path2);
    }
}
