#![allow(clippy::items_after_statements)]
#[cfg(feature = "nova")]
use duke_bytecode::{calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{AttributeData, CpEntry, CpIndex, parse};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
#[cfg(feature = "nova")]
use std::path::Path;
#[cfg(feature = "nova")]
use std::process;

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

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if,
    clippy::case_sensitive_file_extension_comparisons
)]
pub fn dump_similar(jar_path: &str, target_class: &str, target_method: &str) {
    let loader = ZipLoader::open(Path::new(jar_path)).unwrap_or_else(|e| {
        eprintln!("duke: failed to open JAR '{jar_path}': {e}");
        process::exit(1);
    });

    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| name.ends_with(".class"))
        .map(std::string::ToString::to_string)
        .collect();

    let mut target_instructions = None;
    let mut all_methods = Vec::new();

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let class_name = resolve_class_name(&cf, cf.this_class);

                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{class_name}::{name_str}{desc_str}");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(instructions) = decode(&code.code) {
                                let instrs: Vec<_> =
                                    instructions.into_iter().map(|(_, i)| i).collect();
                                if class_name == target_class && name_str == target_method {
                                    target_instructions = Some(instrs.clone());
                                }
                                all_methods.push((full_name.clone(), instrs));
                            }
                        }
                    }
                }
            }
        }
    }

    let Some(target_instrs) = target_instructions else {
        println!("Target method {target_class}::{target_method} not found in {jar_path}.");
        return;
    };

    let mut similarities = Vec::new();
    for (name, instrs) in all_methods {
        let sim = calculate_similarity(&target_instrs, &instrs);
        similarities.push((name, sim));
    }

    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    println!("======================================");
    println!(" JAR Method Similarity Analysis");
    println!("======================================");
    println!("File:                 {jar_path}");
    println!("Target Method:        {target_class}::{target_method}");
    println!();
    println!("--- Top 10 Similar Methods ---");

    for (name, sim) in similarities.iter().take(10) {
        println!("  [{sim:.2}] {name}");
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_similar_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path_str = path.to_str().unwrap();

        dump_similar(path_str, "MainHello", "main");
    }
}
