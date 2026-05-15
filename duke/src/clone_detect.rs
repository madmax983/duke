#![allow(clippy::items_after_statements)]
#![allow(clippy::case_sensitive_file_extension_comparisons)]

#[cfg(feature = "nova")]
use duke_bytecode::{Instruction, calculate_similarity, decode};
#[cfg(feature = "nova")]
use duke_classfile::{
    ClassFile, parse, {AttributeData, CpEntry, CpIndex},
};
#[cfg(feature = "nova")]
use duke_loader::{ClassLoader, ZipLoader};
use std::path::Path;

#[cfg(feature = "nova")]
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn cp_str(cf: &ClassFile, idx: CpIndex) -> Option<&str> {
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
#[cfg(not(tarpaulin_include))]
#[allow(unexpected_cfgs)]
fn resolve_class_name(cf: &ClassFile, idx: CpIndex) -> String {
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
pub fn dump_clone_detect(jar_path: &str) {
    let Ok(loader) = ZipLoader::open(Path::new(jar_path)) else {
        println!("duke: cannot open jar '{jar_path}'");
        return;
    };

    let reader = loader.reader();
    let class_entries: Vec<String> = reader
        .entry_names()
        .filter(|name| name.ends_with(".class"))
        .map(std::string::ToString::to_string)
        .collect();

    let mut decoded_methods: Vec<(String, Vec<Instruction>)> = Vec::new();

    for entry_name in class_entries {
        let class_name_internal = entry_name.strip_suffix(".class").unwrap();
        if let Ok(bytes) = loader.find_class(class_name_internal) {
            if let Ok(cf) = parse(&bytes) {
                let class_name = resolve_class_name(&cf, cf.this_class);

                for method in &cf.methods {
                    let name_str = cp_str(&cf, method.name_index).unwrap_or("<invalid>");
                    let desc_str = cp_str(&cf, method.descriptor_index).unwrap_or("<invalid>");
                    let full_name = format!("{class_name}::{name_str} {desc_str}");

                    for attr in &method.attributes {
                        if let AttributeData::Code(code) = &attr.data {
                            if let Ok(instructions) = decode(&code.code) {
                                let instrs: Vec<Instruction> =
                                    instructions.into_iter().map(|(_, i)| i).collect();

                                if instrs.len() >= 10 {
                                    decoded_methods.push((full_name.clone(), instrs));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("======================================");
    println!(" Code Clone Detection ");
    println!("======================================");
    println!(
        "Analyzing {} methods (length >= 10)...",
        decoded_methods.len()
    );
    println!();

    let mut clones_found = 0;
    for i in 0..decoded_methods.len() {
        for j in (i + 1)..decoded_methods.len() {
            let sim = calculate_similarity(&decoded_methods[i].1, &decoded_methods[j].1);
            if sim >= 0.90 {
                println!("Clone found (Similarity: {sim:.2}):");
                println!("  - {}", decoded_methods[i].0);
                println!("  - {}", decoded_methods[j].0);
                println!();
                clones_found += 1;
            }
        }
    }

    if clones_found == 0 {
        println!("No significant clones found.");
    } else {
        println!("Found {clones_found} potential clone pairs.");
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;

    #[test]
    fn test_dump_clone_detect_dummy() {
        let path = "nonexistent.jar";
        dump_clone_detect(path);
    }

    #[test]
    fn test_dump_clone_detect_valid() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/hello.jar");
        let path = path.to_str().unwrap();

        dump_clone_detect(path);
    }
}
