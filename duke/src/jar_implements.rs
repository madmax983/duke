#![allow(clippy::items_after_statements)]

#[cfg(feature = "nova")]
use duke_classfile::{CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::collections::{HashMap, HashSet, VecDeque};
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

/// Finds all classes in a JAR that implement or extend a given target interface/class.
#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if
)]
pub fn dump_jar_implements(jar_path: &str, target_class: &str) {
    let loader = match ZipLoader::open(Path::new(jar_path)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("duke: failed to open JAR '{jar_path}': {e}");
            return;
        }
    };

    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| name.ends_with(".class"))
        .map(std::string::ToString::to_string)
        .collect();

    // Mapping from a parent (class/interface) to its direct children (subclasses/implementors)
    let mut hierarchy: HashMap<String, HashSet<String>> = HashMap::new();

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let this_name = resolve_class_name(&cf, cf.this_class);

                // Add superclass to hierarchy
                if cf.super_class.0 != 0 {
                    let super_name = resolve_class_name(&cf, cf.super_class);
                    if super_name != "<invalid utf8>"
                        && super_name != "<not a class ref>"
                        && super_name != "<none>"
                    {
                        hierarchy
                            .entry(super_name)
                            .or_default()
                            .insert(this_name.clone());
                    }
                }

                // Add interfaces to hierarchy
                for interface_idx in &cf.interfaces {
                    let interface_name = resolve_class_name(&cf, *interface_idx);
                    if interface_name != "<invalid utf8>"
                        && interface_name != "<not a class ref>"
                        && interface_name != "<none>"
                    {
                        hierarchy
                            .entry(interface_name)
                            .or_default()
                            .insert(this_name.clone());
                    }
                }
            }
        }
    }

    // Perform BFS to find all transitive implementors/subclasses
    let mut implementors = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back(target_class.to_string());

    while let Some(current) = queue.pop_front() {
        if let Some(children) = hierarchy.get(&current) {
            for child in children {
                if implementors.insert(child.clone()) {
                    queue.push_back(child.clone());
                }
            }
        }
    }

    let mut sorted_implementors: Vec<_> = implementors.into_iter().collect();
    sorted_implementors.sort();

    println!("======================================");
    println!(" Transitive Implementation Analysis");
    println!("======================================");
    println!("Target:               {target_class}");
    println!("File:                 {jar_path}");
    println!("Implementors Found:   {}", sorted_implementors.len());
    println!();

    if sorted_implementors.is_empty() {
        println!("No classes found implementing/extending '{target_class}'.");
    } else {
        println!("Found the following implementors:");
        for impl_name in sorted_implementors {
            println!("  - {impl_name}");
        }
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_jar_implements_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        // Object is at the root of everything
        dump_jar_implements(path_str, "java/lang/Object");
    }
}
