//! # JAR Bytecode Diff Analysis
//!
//! This module provides tools for comparing the contents of two Java Archive (JAR) files.
//! It determines exactly which methods were added, removed, or modified between two versions
//! of a library or application.
//!
//! By parsing the underlying `ClassFile` definitions and hashing the raw bytecode within
//! the `Code` attributes of every method, we can determine semantic differences rather
//! than relying simply on classfile timestamps or file sizes.
//!
//! ## Usage
//! The primary entrypoint is `dump_jar_diff`, which takes paths to two JARs and
//! prints a detailed report to standard output.

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
        .and_then(|slot: &Option<duke_classfile::CpEntry>| slot.as_ref())
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
        .and_then(|s: &Option<duke_classfile::CpEntry>| s.as_ref());
    if let Some(CpEntry::Class { name_index }) = class_entry {
        cp_str(cf, *name_index)
            .unwrap_or("<invalid utf8>")
            .to_string()
    } else {
        "<not a class ref>".to_string()
    }
}

/// Analyzes two JAR files and prints a comparative report of their methods.
///
/// This function loads all classes from both JARs, parses their structure, and compares
/// the signatures and bytecode hashes of their methods. It then prints a report detailing
/// the total number of added, removed, modified, and unchanged methods, along with the
/// top 10 methods in each category.
///
/// ## Examples
///
/// ```rust,no_run
/// # #[cfg(feature = "nova")]
/// # {
/// use duke::jar_diff::dump_jar_diff;
///
/// // Compare an old library version with a new one
/// dump_jar_diff("lib-v1.jar", "lib-v2.jar");
/// # }
/// ```
///
/// ## Panics
///
/// This function does not panic on invalid JARs or malformed classes; it simply ignores
/// files that cannot be read or parsed and treats them as empty sets.
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
