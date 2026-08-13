#![allow(clippy::items_after_statements)]
#![allow(clippy::case_sensitive_file_extension_comparisons)]

#[cfg(feature = "nova")]
use duke_bytecode::{calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{
    parse, {AttributeData, CpEntry, CpIndex},
};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
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

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(
    unexpected_cfgs,
    clippy::print_stdout,
    clippy::use_debug,
    clippy::collapsible_if
)]
pub fn dump_jar_clone_detect(jar_path: &str, threshold: f64) {
    let Ok(loader) = ZipLoader::open(Path::new(jar_path)) else {
        eprintln!("duke: warning: failed to open JAR '{}'", jar_path);
        return; // Avoid abrupt exit in tests
    };

    let mut methods_data = Vec::new();

    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| name.ends_with(".class"))
        .map(std::string::ToString::to_string)
        .collect();

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let class_name = resolve_class_name(&cf, cf.this_class);

                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{}::{}{}", class_name, name_str, desc_str);

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(instructions) = decode(&code.code) {
                                let instrs: Vec<_> =
                                    instructions.into_iter().map(|(_, i)| i).collect();
                                if instrs.len() >= 15 {
                                    methods_data.push((full_name.clone(), instrs));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut found = false;
    for i in 0..methods_data.len() {
        for j in (i + 1)..methods_data.len() {
            let sim = calculate_similarity(&methods_data[i].1, &methods_data[j].1);
            if sim >= threshold {
                if !found {
                    println!("======================================");
                    println!(" JAR Structural Clones Detected (Threshold: {:.2})", threshold);
                    println!("======================================");
                    found = true;
                }
                println!("Similarity: {:.2}", sim);
                println!("  - {}", methods_data[i].0);
                println!("  - {}", methods_data[j].0);
                println!();
            }
        }
    }

    if !found {
        println!("No structural clones found with similarity >= {:.2}", threshold);
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_jar_clone_detect_dummy() {
        // Will just print a warning and return early since dummy.jar doesn't exist
        dump_jar_clone_detect("dummy.jar", 0.9);
    }
}
